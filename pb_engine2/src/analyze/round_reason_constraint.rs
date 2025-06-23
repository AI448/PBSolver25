use ordered_float::OrderedFloat;

use crate::{
    Literal,
    analyze::utility::lhs_sup_of_linear_constraint_at,
    constraint::{LinearConstraint, LinearConstraintTrait, RandomLinearConstraint},
    pb_engine::PBEngine,
};

use super::{identify_propagation_causals::IdentifyPropagationCausals, round::Round2};

pub struct RoundReasonConstraint {
    integrality_tolerance: f64,
    identify_causals: IdentifyPropagationCausals,
    // weaken: Weaken,
    round: Round2<u64>,
    linear_constraint: LinearConstraint<u64>,
}

impl RoundReasonConstraint {
    pub fn new(integrality_tolerance: f64) -> Self {
        Self {
            integrality_tolerance,
            identify_causals: IdentifyPropagationCausals::new(),
            // weaken: Weaken::new(),
            round: Round2::new(),
            linear_constraint: LinearConstraint::default(),
        }
    }

    pub fn round(
        &mut self,
        reason_constraint: &impl LinearConstraintTrait<Value = u64>,
        conflict_constraint: &RandomLinearConstraint<u128>,
        propagated_assignment: Literal,
        pb_engine: &PBEngine,
    ) -> impl LinearConstraintTrait<Value = u64> + '_ {
        assert!(
            reason_constraint
                .iter_terms()
                .find(|&(literal, _)| literal == propagated_assignment)
                .is_some()
        );
        assert!(conflict_constraint.get(!propagated_assignment).is_some());

        // 伝播の原因となった割り当てを特定
        let (causal_assignments, _slack) = self.identify_causals.call(
            reason_constraint,
            propagated_assignment,
            |literal| {
                // (
                //     // conflict_constraint に含まれるリテラルを優先
                //     if conflict_constraint.get(!literal).is_some() {
                //         2
                //     } else if conflict_constraint.get(literal).is_some() {
                //         1
                //     } else {
                //         0
                //     },
                // 割り当て順が早いリテラルを優先
                // usize::MAX - pb_engine.get_assignment_order(literal.index()),
                // アクティビティが大きいものを優先
                OrderedFloat::from(pb_engine.activity(literal.index()))
                // )
            },
            pb_engine,
        );
        debug_assert!(causal_assignments.iter().all(|literal| pb_engine.is_true(literal)));

        let divisor = reason_constraint
            .iter_terms()
            .find(|&(literal, _)| literal == propagated_assignment)
            .unwrap()
            .1;
        let multipler = conflict_constraint.get(!propagated_assignment).unwrap();

        let hoge = reason_constraint.convert();
        // round
        let rounded_reason_constraint = self.round.calculate(
            &hoge,
            divisor,
            |literal| causal_assignments.contains_key(!literal),
            move |literal| {
                conflict_constraint
                    .get(!literal)
                    .map(|anticoefficient| anticoefficient as f64 / multipler as f64)
                    .unwrap_or(0.0)
            },
            pb_engine,
        );
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                rounded_reason_constraint
                    .iter_terms()
                    .find(|&(literal, _)| literal == propagated_assignment)
                    .unwrap()
                    .1
                    == 1
            );
            let sup_at_propaged = lhs_sup_of_linear_constraint_at(
                &rounded_reason_constraint,
                pb_engine.get_assignment_order(propagated_assignment.index()),
                pb_engine,
            );
            debug_assert!(sup_at_propaged < rounded_reason_constraint.lower() + 1);
        }

        // return rounded_reason_constraint;

        self.linear_constraint.replace_by_linear_constraint(&rounded_reason_constraint);
        return self.linear_constraint.as_view();
    }
}
