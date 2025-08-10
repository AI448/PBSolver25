use num::integer::gcd;

use crate::{
    LinearConstraintTrait, PBEngine,
    analyze::utility::{StrengthenLinearConstraint, lhs_sup_of_linear_constraint_at},
    constraints::RandomAccessibleLinearConstraint,
};

use super::round_reason_constraint::RoundReasonConstraint;

pub struct Resolve {
    strengthen: StrengthenLinearConstraint,
    round_constraint: RoundReasonConstraint,
    resolved_constraint: RandomAccessibleLinearConstraint<u128>,
}

impl Resolve {
    pub fn new(integrality_tolerance: f64) -> Self {
        Self {
            strengthen: StrengthenLinearConstraint::default(),
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

        let conflict_sup =
            lhs_sup_of_linear_constraint_at(&conflict_constraint, conflict_order - 1, engine);
        let reason_slack =
            lhs_sup_of_linear_constraint_at(&reason_constraint, conflict_order - 1, engine)
                - reason_constraint.lower();
        debug_assert!(reason_slack < reason_coefficient);

        if conflict_sup < conflict_constraint.lower() {
            if reason_slack == 0 || conflict_coefficient == 1 {
                self.resolved_constraint.replace_by_linear_constraint(
                    conflict_constraint
                        .convert()
                        .mul(reason_coefficient as u128),
                );
                self.resolved_constraint.add_assign(
                    reason_constraint
                        .convert()
                        .mul(conflict_coefficient as u128),
                );
            } else {
                self.resolved_constraint
                    .replace_by_linear_constraint(conflict_constraint.convert());

                let rounded_reason_constraint = self.round_constraint.round(
                    reason_constraint,
                    &self.resolved_constraint,
                    propagated_literal,
                    engine,
                );

                self.resolved_constraint.add_assign(
                    rounded_reason_constraint
                        .convert()
                        .mul(conflict_coefficient as u128),
                );
            }
        } else {
            let conflict_slack = conflict_sup - conflict_constraint.lower();
            // if (conflict_slack as f64) / (conflict_coefficient as f64) + (reason_slack as f64) / (reason_coefficient as f64) < 0.9999 {
            if (conflict_slack as u128) * (reason_coefficient as u128)
                + (reason_slack as u128) * (conflict_coefficient as u128)
                < (conflict_coefficient as u128) * (reason_coefficient as u128)
            {
                // if conflict_slack == 0 || reason_slack == 0 {
                // if conflict_slack == 0 && reason_slack == 0 {
                let g = gcd(conflict_coefficient, reason_coefficient);
                self.resolved_constraint.replace_by_linear_constraint(
                    conflict_constraint
                        .convert()
                        .mul((reason_coefficient / g) as u128),
                );
                self.resolved_constraint.add_assign(
                    reason_constraint
                        .convert()
                        .mul((conflict_coefficient / g) as u128),
                );
            } else {
                // MEMO: どちらを丸めても大して変わらない？
                // slack が小さい方を丸める
                // if (conflict_slack as u128) * (reason_coefficient as u128) > (reason_slack as u128) * (conflict_coefficient as u128) {
                // slack が大きい方を丸める
                if (conflict_slack as u128) * (reason_coefficient as u128)
                    < (reason_slack as u128) * (conflict_coefficient as u128)
                {
                    // 係数が小さい方を丸める
                    // if reason_coefficient <= conflict_coefficient {
                    // 係数が大きい方を丸める
                    // if reason_coefficient >= conflict_coefficient {
                    // if true {
                    self.resolved_constraint
                        .replace_by_linear_constraint(conflict_constraint.convert());

                    let rounded_reason_constraint = self.round_constraint.round(
                        reason_constraint,
                        &self.resolved_constraint,
                        propagated_literal,
                        engine,
                    );

                    self.resolved_constraint.add_assign(
                        rounded_reason_constraint
                            .convert()
                            .mul(conflict_coefficient as u128),
                    );
                } else {
                    self.resolved_constraint
                        .replace_by_linear_constraint(reason_constraint.convert());

                    let rounded_conflict_constraint = self.round_constraint.round(
                        conflict_constraint,
                        &self.resolved_constraint,
                        !propagated_literal,
                        engine,
                    );

                    self.resolved_constraint.add_assign(
                        rounded_conflict_constraint
                            .convert()
                            .mul(reason_coefficient as u128),
                    );
                }
            }
        }

        return self.strengthen.strengthen(&self.resolved_constraint);
    }
}
