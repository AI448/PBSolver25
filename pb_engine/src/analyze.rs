mod calculate_propagation_level;
mod flatten;
mod identify_conflict_causals;
mod identify_propagation_causals;
mod resolve;
mod round;
mod round_reason_constraint;
mod utility;
mod weaken;

use std::{cmp::Reverse, usize};

use calculate_propagation_level::CalculatePropagationLevel;
use flatten::FlattenConflictConstraint;
use identify_propagation_causals::IdentifyPropagationCausals;
use resolve::Resolve;
use utility::{drop_fixed_variable, lhs_sup_of_linear_constraint_at};

use crate::{
    Boolean, LinearConstraint, LinearConstraintTrait, Literal, PBEngine, PBExplainKey, Reason,
    collections::LiteralSet,
};

// TODO: learnt_constraint は LinearConstraint でいい
pub enum AnalyzeResult<LinearConstraintT, ConflictingAssignmentsT>
where
    LinearConstraintT: LinearConstraintTrait<Value = u64>,
    ConflictingAssignmentsT: Iterator<Item = Literal>,
{
    Backjumpable {
        backjump_level: usize,
        learnt_constraint: LinearConstraintT,
        conflicting_assignments: ConflictingAssignmentsT,
    },
    Unsatisfiable,
}

pub struct Analyze {
    calculate_propagation_level: CalculatePropagationLevel,
    identify_propagation_causals: IdentifyPropagationCausals,
    resolve: Resolve,
    flatten: FlattenConflictConstraint,
    conflicting_assignments: LiteralSet,
    conflict_constraint: LinearConstraint<u64>,
}

impl Analyze {
    pub fn new(integrality_tolerance: f64) -> Self {
        Self {
            calculate_propagation_level: CalculatePropagationLevel::new(),
            identify_propagation_causals: IdentifyPropagationCausals::new(),
            resolve: Resolve::new(integrality_tolerance),
            flatten: FlattenConflictConstraint::new(1000000),
            conflicting_assignments: LiteralSet::default(),
            conflict_constraint: LinearConstraint::default(),
        }
    }
    pub fn call(
        &mut self,
        conflict_variable: usize,
        conflict_explain_keys: [PBExplainKey; 2],
        engine: &PBEngine,
    ) -> AnalyzeResult<
        impl LinearConstraintTrait<Value = u64> + '_,
        impl Iterator<Item = Literal> + '_,
    > {
        self.conflicting_assignments.clear();
        // TODO: 意味があるのか確認
        // self.conflicting_assignments
        //     .insert(Literal::new(conflict_variable, Boolean::FALSE));
        // self.conflicting_assignments
        //     .insert(Literal::new(conflict_variable, Boolean::TRUE));

        // conflict_constraint を初期化
        {
            // 矛盾している制約条件の組を取得
            let conflict_constraints =
                conflict_explain_keys.map(|explain_key| engine.explain(explain_key));
            // それぞれの係数を取得
            let conflict_coefficients = [Boolean::FALSE, Boolean::TRUE].map(|value| {
                conflict_constraints[value]
                    .iter_terms()
                    .find(|&(literal, _)| literal == Literal::new(conflict_variable, value))
                    .unwrap()
                    .1
            });
            // 各制約条件のconflicting_variable の係数で正規化したスラックを算出
            let slack = [Boolean::FALSE, Boolean::TRUE].map(|value| {
                self.identify_propagation_causals
                    .call(
                        &conflict_constraints[value],
                        Literal::new(conflict_variable, value),
                        |literal| Reverse(engine.get_assignment_order(literal.index())),
                        engine,
                    )
                    .1
                    / conflict_coefficients[value]
            });
            // スラックが大きい方を c, 小さい方を r とする
            let (c, r) = if slack[0] >= slack[1] { (0, 1) } else { (1, 0) };
            // 矛盾している制約条件を融合して conflict_constraint とする (r の方が丸められる)
            self.conflict_constraint.replace(self.resolve.call(
                &drop_fixed_variable(&conflict_constraints[c], engine),
                &drop_fixed_variable(&conflict_constraints[r], engine),
                conflict_variable,
                engine,
            ));
        }

        let mut conflict_order = usize::MAX;
        loop {
            #[cfg(debug_assertions)]
            {
                let sup = lhs_sup_of_linear_constraint_at(
                    &self.conflict_constraint,
                    conflict_order,
                    engine,
                );
                eprintln!(
                    "sup={}, lower={}, len={}",
                    sup,
                    self.conflict_constraint.lower(),
                    self.conflict_constraint.len()
                );
                debug_assert!(sup < self.conflict_constraint.lower());
            }

            let sup0: u64 = self
                .conflict_constraint
                .iter_terms()
                .map(|(_, coefficient)| coefficient)
                .sum();
            if sup0 < self.conflict_constraint.lower() {
                return AnalyzeResult::Unsatisfiable;
            }

            if let Some(_) =
                self.calculate_propagation_level
                    .call(&self.conflict_constraint, engine, false)
            {
                let backjump_level = self
                    .calculate_propagation_level
                    .call(&self.conflict_constraint, engine, true)
                    .unwrap();
                for (literal, _) in self.conflict_constraint.iter_terms() {
                    if engine.is_false(literal) {
                        self.conflicting_assignments.insert(literal);
                    }
                }
                return AnalyzeResult::Backjumpable {
                    backjump_level: backjump_level,
                    learnt_constraint: &self.conflict_constraint,
                    conflicting_assignments: self.conflicting_assignments.iter(),
                };
            }

            let resolving_variable = self
                .conflict_constraint
                .iter_terms()
                .map(|(literal, _)| literal)
                .filter(|&literal| engine.is_false_at(literal, conflict_order - 1) && engine.get_reason(literal.index()).unwrap().is_propagation())
                .max_by_key(|&literal| engine.get_assignment_order(literal.index()))
                .unwrap()
                .index();

            conflict_order = engine.get_assignment_order(resolving_variable);

            let reason_constraint = {
                let Reason::Propagation { explain_key } =
                    engine.get_reason(resolving_variable).unwrap()
                else {
                    unreachable!()
                };
                engine.explain(explain_key)
            };

            let resolved_constraint = self.resolve.call(
                &self.conflict_constraint,
                &drop_fixed_variable(&reason_constraint, engine),
                resolving_variable,
                engine,
            );

            let max_coefficient = resolved_constraint.iter_terms().map(|(_, coefficient)| coefficient).max().unwrap_or(0);
            if max_coefficient <= 1000000 {
                self.conflict_constraint.replace(&resolved_constraint);
            } else {
                let flattened_constraint = self.flatten.call(&resolved_constraint, conflict_order, engine);
                self.conflict_constraint.replace(&flattened_constraint);
            }
            
        }
    }
}
