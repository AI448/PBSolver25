use num::integer::gcd;

use crate::{
    StrengthenConstraint,
    analyze::utility::lhs_sup_of_linear_constraint_at,
    constraint::{LinearConstraintTrait, RandomLinearConstraint},
    pb_engine::PBEngine,
};

use super::round_reason_constraint::RoundReasonConstraint;

pub struct Resolve {
    round_constraint: RoundReasonConstraint,
    streangthen: StrengthenConstraint<u128>,
    resolved_constraint: RandomLinearConstraint<u128>,
}

impl Resolve {
    pub fn new() -> Self {
        Self {
            round_constraint: RoundReasonConstraint::new(),
            streangthen: StrengthenConstraint::default(),
            resolved_constraint: RandomLinearConstraint::default(),
        }
    }
    pub fn call(
        &mut self,
        conflict_constraint: &impl LinearConstraintTrait<Value = u64>,
        reason_constraint: &impl LinearConstraintTrait<Value = u64>,
        resolving_variable: usize,
        engine: &PBEngine,
    ) -> impl LinearConstraintTrait<Value = u128> + '_ {
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

        let conflict_coefficient = conflict_constraint
            .iter_terms()
            .find(|&(literal, _)| literal.index() == resolving_variable)
            .unwrap()
            .1;

        let reason_coefficient = reason_constraint
            .iter_terms()
            .find(|&(literal, _)| literal.index() == resolving_variable)
            .unwrap()
            .1;

        let conflict_slack =
            lhs_sup_of_linear_constraint_at(conflict_constraint, conflict_order - 1, engine)
                as i128
                - conflict_constraint.lower() as i128;
        let reason_slack =
            lhs_sup_of_linear_constraint_at(reason_constraint, conflict_order - 1, engine) as i128
                - reason_constraint.lower() as i128;

        if conflict_slack * (reason_coefficient as i128)
            + reason_slack * (conflict_coefficient as i128)
            < (conflict_coefficient as i128) * (reason_coefficient as i128)
        {
            let g = gcd(conflict_coefficient, reason_coefficient);
            self.resolved_constraint.replace_by_constraint(
                conflict_constraint.convert().mul((reason_coefficient / g) as u128),
            );
            self.resolved_constraint
                .add_assign(&reason_constraint.convert().mul((conflict_coefficient / g) as u128));
        } else {
            // slack が大きい方を丸める
            if conflict_slack * (reason_coefficient as i128)
                < reason_slack * (conflict_coefficient as i128)
            {
                self.resolved_constraint.replace_by_constraint(conflict_constraint.convert());

                let rounded_reason_constraint = self.round_constraint.round(
                    reason_constraint,
                    &self.resolved_constraint,
                    propagated_literal,
                    engine,
                );

                self.resolved_constraint.add_assign(
                    &rounded_reason_constraint.convert().mul(conflict_coefficient as u128),
                );
            } else {
                self.resolved_constraint.replace_by_constraint(reason_constraint.convert());

                let rounded_conflict_constraint = self.round_constraint.round(
                    conflict_constraint,
                    &self.resolved_constraint,
                    !propagated_literal,
                    engine,
                );

                self.resolved_constraint.add_assign(
                    &rounded_conflict_constraint.convert().mul(reason_coefficient as u128),
                );
            }
        }

        return self.streangthen.exec(&self.resolved_constraint, engine);
    }
}
