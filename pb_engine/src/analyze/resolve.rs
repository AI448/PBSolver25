use crate::{
    analyze::utility::strengthen_integer_linear_constraint, constraints::RandomAccessibleLinearConstraint, LinearConstraintTrait, Literal, PBEngine
};

use super::round_reason_constraint::RoundReasonConstraint;

pub struct Resolve {
    round_reason_constraint: RoundReasonConstraint,
    conflict_constraint: RandomAccessibleLinearConstraint<u64>,
}

impl Resolve {
    pub fn new(integrality_tolerance: f64) -> Self {
        Self {
            round_reason_constraint: RoundReasonConstraint::new(integrality_tolerance),
            conflict_constraint: RandomAccessibleLinearConstraint::default(),
        }
    }
    pub fn call(
        &mut self,
        conflict_constraint: &impl LinearConstraintTrait<Value = u64>,
        reason_constraint: &impl LinearConstraintTrait<Value = u64>,
        resolving_variable: usize,
        engine: &PBEngine,
    ) -> impl LinearConstraintTrait<Value = u64> + '_ {

        let propagated_literal = reason_constraint.iter_terms().find(|&(literal, _)| literal.index() == resolving_variable).unwrap().0;

        debug_assert!(conflict_constraint.iter_terms().find(|&(literal, _)| literal == !propagated_literal).is_some());

        self.conflict_constraint
            .replace_by_linear_constraint(&conflict_constraint);
        let conflict_coefficient = self
            .conflict_constraint
            .get(Literal::new(resolving_variable, crate::Boolean::FALSE))
            .or(self
                .conflict_constraint
                .get(Literal::new(resolving_variable, crate::Boolean::TRUE)))
            .unwrap();

        self.round_reason_constraint.round(
            reason_constraint,
            &self.conflict_constraint,
            propagated_literal,
            engine,
        );

        self.conflict_constraint
            .add_assign(&self.round_reason_constraint.get().mul(conflict_coefficient));

        return strengthen_integer_linear_constraint(&self.conflict_constraint);
    }
}
