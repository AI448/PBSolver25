use num::{integer::gcd, Integer};

use crate::{
    analyze::utility::{lhs_sup_of_linear_constraint_at, strengthen_integer_linear_constraint}, constraints::RandomAccessibleLinearConstraint, LinearConstraintTrait, Literal, PBEngine
};

use super::round_reason_constraint::RoundReasonConstraint;

pub struct Resolve {
    round_constraint: RoundReasonConstraint,
    resolved_constraint: RandomAccessibleLinearConstraint<u64>,
}

impl Resolve {
    pub fn new(integrality_tolerance: f64) -> Self {
        Self {
            round_constraint: RoundReasonConstraint::new(integrality_tolerance),
            resolved_constraint: RandomAccessibleLinearConstraint::default(),
        }
    }
    pub fn call(
        &mut self,
        conflict_constraint: &impl LinearConstraintTrait<Value = u64>,
        reason_constraint: &impl LinearConstraintTrait<Value = u64>,
        resolving_variable: usize,
        engine: &PBEngine,
    ) -> impl LinearConstraintTrait<Value = u64> + '_ {
        let propagated_literal = reason_constraint
            .iter_terms()
            .find(|&(literal, _)| literal.index() == resolving_variable)
            .unwrap()
            .0;
        debug_assert!(
            conflict_constraint
                .iter_terms()
                .find(|&(literal, _)| literal == !propagated_literal)
                .is_some()
        );

        let conflict_order = engine.get_assignment_order(propagated_literal.index());

        let conflict_slack = lhs_sup_of_linear_constraint_at(&conflict_constraint, conflict_order - 1, engine) - conflict_constraint.lower();
        let reason_slack = lhs_sup_of_linear_constraint_at(&reason_constraint, conflict_order - 1, engine) - reason_constraint.lower();

        if conflict_slack == 0 || reason_slack == 0 {
            let conflict_coefficient = conflict_constraint.iter_terms().find(|&(literal, _)|literal.index() == resolving_variable).unwrap().1;
            let reason_coefficient = reason_constraint.iter_terms().find(|&(literal, _)|literal.index() == resolving_variable).unwrap().1;
            let g = gcd(conflict_coefficient, reason_coefficient);
            self.resolved_constraint
                .replace_by_linear_constraint(&conflict_constraint.mul(reason_coefficient / g));
            self.resolved_constraint.add_assign(
                &reason_constraint.mul(conflict_coefficient / g)
            );
        } else {
            self.resolved_constraint
                .replace_by_linear_constraint(&conflict_constraint);
            let conflict_coefficient = self
                .resolved_constraint
                .get(Literal::new(resolving_variable, crate::Boolean::FALSE))
                .or(self
                    .resolved_constraint
                    .get(Literal::new(resolving_variable, crate::Boolean::TRUE)))
                .unwrap();

            self.round_constraint.round(
                reason_constraint,
                &self.resolved_constraint,
                propagated_literal,
                engine,
            );

            self.resolved_constraint
                .add_assign(&self.round_constraint.get().mul(conflict_coefficient));
        }

        return strengthen_integer_linear_constraint(&self.resolved_constraint);
    }
}
