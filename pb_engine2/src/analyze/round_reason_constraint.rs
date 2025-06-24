use crate::{
    Literal,
    analyze::utility::lhs_sup_of_linear_constraint_at,
    constraint::{LinearConstraint, LinearConstraintTrait, RandomLinearConstraint},
    pb_engine::PBEngine,
};

use super::round::Round2;

pub struct RoundReasonConstraint {
    round: Round2<u64>,
    linear_constraint: LinearConstraint<u64>,
}

impl RoundReasonConstraint {
    pub fn new() -> Self {
        Self {
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

        let divisor = reason_constraint
            .iter_terms()
            .find(|&(literal, _)| literal == propagated_assignment)
            .unwrap()
            .1;

        // round
        let rounded_reason_constraint = self.round.calculate(
            &reason_constraint.convert(),
            divisor,
            |literal| {
                pb_engine.is_false(literal)
                    && pb_engine.get_assignment_order(literal.index())
                        < pb_engine.get_assignment_order(propagated_assignment.index())
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
        self.linear_constraint.replace_by_linear_constraint(&rounded_reason_constraint);
        return self.linear_constraint.as_view();
    }
}
