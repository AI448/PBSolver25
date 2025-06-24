mod flatten;
mod resolve;
mod round;
mod round_reason_constraint;
mod utility;

use flatten::FlattenConflictConstraint;
use resolve::Resolve;
use utility::{drop_fixed_variable, lhs_sup_of_linear_constraint_at};

use crate::{
    Literal,
    collections::LiteralSet,
    constraint::{ConstraintView, LinearConstraint, LinearConstraintTrait},
    pb_engine::{PBEngine, PBExplainKey, Reason},
    utility::calculate_propagation_level::CalculatePropagationLevel,
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
    resolve: Resolve,
    flatten: FlattenConflictConstraint,
    conflicting_assignments: LiteralSet,
    conflict_constraint: LinearConstraint<u64>,
}

impl Analyze {
    pub fn new() -> Self {
        Self {
            calculate_propagation_level: CalculatePropagationLevel::new(),
            resolve: Resolve::new(),
            flatten: FlattenConflictConstraint::new(u32::MAX as u64),
            conflicting_assignments: LiteralSet::default(),
            conflict_constraint: LinearConstraint::default(),
        }
    }
    pub fn call(
        &mut self,
        // conflict_variable: usize,
        conflict_explain_key: PBExplainKey,
        engine: &PBEngine,
    ) -> AnalyzeResult<
        impl LinearConstraintTrait<Value = u64> + '_,
        impl Iterator<Item = Literal> + '_,
    > {
        eprintln!("ANALYZE");

        self.conflicting_assignments.clear();
        // TODO: 意味があるのか確認

        // conflict_constraint を初期化
        self.conflict_constraint.replace_by_linear_constraint(&drop_fixed_variable(
            &engine.explain(conflict_explain_key),
            engine,
        ));

        let mut conflict_order = usize::MAX;
        loop {
            #[cfg(debug_assertions)]
            {
                let sup = lhs_sup_of_linear_constraint_at(
                    &self.conflict_constraint,
                    conflict_order,
                    engine,
                );
                // eprintln!(
                //     "sup={}, lower={}, {:?}",
                //     sup,
                //     self.conflict_constraint.lower(),
                //     self.conflict_constraint
                // );
                debug_assert!(sup < self.conflict_constraint.lower());
            }

            let sup0: u64 =
                self.conflict_constraint.iter_terms().map(|(_, coefficient)| coefficient).sum();
            if sup0 < self.conflict_constraint.lower() {
                return AnalyzeResult::Unsatisfiable;
            }

            if let Some(_) =
                self.calculate_propagation_level.call(&self.conflict_constraint, engine, false)
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
                    learnt_constraint: ConstraintView::new(
                        self.conflict_constraint.iter_terms(),
                        self.conflict_constraint.lower(),
                    ),
                    conflicting_assignments: self.conflicting_assignments.iter(),
                };
            }

            // let conflict_literal = self
            //     .find_conflict_literal
            //     .find(&self.conflict_constraint, engine);

            let conflict_literal = self
                .conflict_constraint
                .iter_terms()
                .filter(|&(literal, _)| {
                    engine.is_false_at(literal, conflict_order)
                        && engine.get_reason(literal.index()).unwrap().is_propagation()
                })
                .max_by_key(|&(literal, _)| engine.get_assignment_order(literal.index()))
                .unwrap()
                .0;

            conflict_order = engine.get_assignment_order(conflict_literal.index());

            let reason_constraint = {
                let Reason::Propagation { explain_key } =
                    engine.get_reason(conflict_literal.index()).unwrap()
                else {
                    unreachable!()
                };
                engine.explain(explain_key)
            };
            let reason_constraint = drop_fixed_variable(&reason_constraint, engine);

            let resolved_constraint = self.resolve.call(
                &self.conflict_constraint,
                &reason_constraint,
                conflict_literal.index(),
                engine,
            );

            self.conflict_constraint.replace_by_linear_constraint(
                &self.flatten.call(&resolved_constraint, conflict_order, engine).convert(),
            );

            self.conflicting_assignments.insert(!conflict_literal);
        }
    }
}
