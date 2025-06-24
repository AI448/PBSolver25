use num::Num;
use std::ops::AddAssign;

use crate::constraint::{ConstraintView, LinearConstraintTrait};
use crate::pb_engine::PBEngine;

pub fn drop_fixed_variable(
    constraint: &impl LinearConstraintTrait<Value = u64>,
    engine: &PBEngine,
) -> impl LinearConstraintTrait<Value = u64> {
    let mut lower = constraint.lower();
    for (literal, coefficient) in constraint.iter_terms() {
        if engine.is_true(literal) && engine.get_decision_level(literal.index()) == 0 {
            lower -= coefficient;
        }
    }
    return ConstraintView::new(
        constraint
            .iter_terms()
            .filter(|(literal, _)| engine.get_decision_level(literal.index()) != 0),
        lower,
    );
}

pub fn lhs_sup_of_linear_constraint_at<ValueT>(
    constraint: &impl LinearConstraintTrait<Value = ValueT>,
    order: usize,
    engine: &PBEngine,
) -> ValueT
where
    ValueT: Num + AddAssign + Copy,
{
    let mut sup = ValueT::zero();
    for (literal, coefficient) in constraint.iter_terms() {
        if !engine.is_false_at(literal, order) {
            sup += coefficient;
        }
    }
    return sup;
}
