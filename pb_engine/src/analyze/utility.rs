use std::ops::AddAssign;

use either::Either;
use num::Num;

use crate::{
      LinearConstraintTrait, LinearConstraintView,
    Literal,  PBEngine,
};

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
    return LinearConstraintView::new(
        constraint
            .iter_terms()
            .filter(|(literal, _)| engine.get_decision_level(literal.index()) != 0),
        lower,
    );
}

pub fn divide_linear_constraint(
    constraint: &impl LinearConstraintTrait<Value = u64>,
    divisor: f64,
) -> impl LinearConstraintTrait<Value = f64> + '_ {
    return LinearConstraintView::new(
        constraint
            .iter_terms()
            .map(move |(literal, coefficient)| (literal, coefficient as f64 / divisor)),
        constraint.lower() as f64 / divisor,
    );
}

pub fn normalize_linear_constraint<'a>(
    constraint: &'a impl LinearConstraintTrait<Value = u64>,
    target_literal: Literal,
) -> impl LinearConstraintTrait<Value = f64> + 'a {
    let target_coefficient = constraint
        .iter_terms()
        .find(|&(literal, _)| literal == target_literal)
        .unwrap()
        .1;
    return LinearConstraintView::new(
        constraint.iter_terms().map(move |(literal, coefficient)| {
            (literal, coefficient as f64 / target_coefficient as f64)
        }),
        constraint.lower() as f64 / target_coefficient as f64,
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

pub fn strengthen_integer_linear_constraint(
    constraint: &impl LinearConstraintTrait<Value = u64>,
) -> impl LinearConstraintTrait<Value = u64> {
    let lower = constraint.lower();

    let mut sum_of_unsaturating_coefficients = 0;
    for (_, coefficient) in constraint.iter_terms() {
        if coefficient < lower {
            sum_of_unsaturating_coefficients += coefficient;
        }
    }

    if sum_of_unsaturating_coefficients < lower {
        return Either::Left(LinearConstraintView::new(
            constraint
                .iter_terms()
                .filter(move |&(_, coefficient)| coefficient >= lower),
            1,
        ));
    } else {
        // TODO: coefficient = min(coefficient, lower)
        // TODO: calculate GCD
        return Either::Right(LinearConstraintView::new(
            constraint.iter_terms(),
            constraint.lower(),
        ));
    }
}
