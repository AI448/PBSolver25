use super::round::Round2;
use crate::constraint::LinearConstraintTrait;
use crate::pb_engine::PBEngine;
use either::Either;
use std::cmp::{Reverse, max};

#[derive(Clone)]
pub struct FlattenConflictConstraint {
    threshold: u64,
    round: Round2<u128>,
    work: Vec<u128>,
}

impl FlattenConflictConstraint {
    pub fn new(threshold: u64) -> Self {
        Self {
            threshold,
            round: Round2::new(),
            work: Vec::default(),
        }
    }

    pub fn call<'a>(
        &'a mut self,
        conflict_constraint: &'a impl LinearConstraintTrait<Value = u128>,
        conflict_order: usize,
        engine: &'a PBEngine,
    ) -> impl LinearConstraintTrait<Value = u128> + 'a {
        let max_coefficient =
            conflict_constraint.iter_terms().map(|(_, coefficient)| coefficient).max().unwrap_or(0);
        if max_coefficient <= self.threshold as u128 {
            return Either::Left(conflict_constraint);
        }

        self.work.clear();
        self.work.extend(
            conflict_constraint
                .iter_terms()
                .filter(|&(literal, _)| {
                    engine.is_false(literal)
                        && engine.get_assignment_order(literal.index()) < conflict_order
                })
                .map(|(_, coefficient)| coefficient),
        );
        self.work.sort_unstable_by_key(|&coefficient| Reverse(coefficient));

        let mut sum_of_coefficients: u128 =
            conflict_constraint.iter_terms().map(|(_, coefficient)| coefficient).sum();
        let mut divisor = 0;

        for &coefficient in self.work.iter() {
            if sum_of_coefficients < conflict_constraint.lower() {
                divisor = coefficient;
                break;
            }
            sum_of_coefficients -= coefficient;
        }

        let divisor = max(max_coefficient.div_ceil(self.threshold as u128), divisor);
        // eprintln!(
        //     "FLATTEN max_coefficient={}, min_causal_coefficient={}, divisor={}",
        //     max_coefficient, min_causal_coefficient, divisor
        // );

        // round
        let rounded_conflict_constraint = self.round.calculate(
            conflict_constraint,
            divisor,
            move |literal| {
                engine.is_false(literal)
                    && engine.get_assignment_order(literal.index()) < conflict_order
            },
            engine,
        );
        return Either::Right(rounded_conflict_constraint);
    }
}
