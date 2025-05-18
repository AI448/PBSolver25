use std::cmp::{Reverse, max, min};
use std::u64;

use ordered_float::OrderedFloat;

use crate::analyze::utility::divide_linear_constraint;
use crate::{LinearConstraintTrait, PBEngine};

use super::identify_conflict_causals::IdentifyConflictCausals;
use super::round::Round;
use super::weaken::Weaken;

#[derive(Clone)]
pub struct FlattenConflictConstraint {
    threshold: u64,
    identify_causals: IdentifyConflictCausals,
    weaken: Weaken,
    round: Round,
    // linear_constraint: LinearConstraint<u64>,
}

impl FlattenConflictConstraint {
    pub fn new(threshold: u64) -> Self {
        Self {
            threshold,
            identify_causals: IdentifyConflictCausals::default(),
            weaken: Weaken::new(),
            round: Round::new(1e-7),
            // linear_constraint: LinearConstraint::default(),
        }
    }

    pub fn call<'a>(
        &'a mut self,
        conflict_constraint: &'a impl LinearConstraintTrait<Value = u64>,
        conflict_order: usize,
        engine: &PBEngine,
    ) -> impl LinearConstraintTrait<Value = u64> + 'a {
        // 矛盾の原因となっている割り当てを特定
        let (causals, _) = self.identify_causals.call(
            &conflict_constraint,
            conflict_order,
            |literal, _| {
                // 割り当て順序が早いものを優先
                // Reverse(engine.get_assignment_order(literal.index()))
                // アクティビティが大きいものを優先
                OrderedFloat::from(engine.activity(literal.index()))
            },
            engine,
        );
        debug_assert!(
            causals
                .iter()
                .all(|literal| engine.is_true_at(literal, conflict_order))
        );

        // weaken
        let weakened_conflict_constraint = self.weaken.call(
            &conflict_constraint,
            |literal| {
                if causals.contains_key(!literal) {
                    None
                } else {
                    Some(0)
                }
            },
            engine,
        );

        // 係数の範囲を算出
        let (_, max_coefficient) = Self::calculate_coefficient_range(&weakened_conflict_constraint);
        let min_causal_coefficient = weakened_conflict_constraint
            .iter_terms()
            .filter(|&(literal, _)| causals.contains_key(!literal))
            .map(|(_, coefficient)| coefficient)
            .min()
            .unwrap();

        let divisor = max(max_coefficient / self.threshold, min_causal_coefficient);
        eprintln!(
            "FLATTEN max_coefficient={}, min_causal_coefficient={}, divisor={}",
            max_coefficient, min_causal_coefficient, divisor
        );

        let normalized_conflict_constraint =
            divide_linear_constraint(&weakened_conflict_constraint, divisor as f64);

        // round
        self.round.calculate(
            &normalized_conflict_constraint,
            |literal| causals.contains_key(!literal),
            |_| 0.0,
            engine,
        );
        return self.round.get();
    }

    fn calculate_coefficient_range(
        constraint: &impl LinearConstraintTrait<Value = u64>,
    ) -> (u64, u64) {
        let mut max_coefficient = 0;
        let mut min_coefficient = u64::MAX;
        for (_, coefficient) in constraint.iter_terms() {
            max_coefficient = max(max_coefficient, coefficient);
            min_coefficient = min(min_coefficient, coefficient);
        }
        return (min_coefficient, max_coefficient);
    }
}
