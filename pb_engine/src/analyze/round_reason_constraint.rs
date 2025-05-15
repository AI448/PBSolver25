use ordered_float::OrderedFloat;

use crate::{
    LinearConstraintTrait, Literal, PBEngine,
    analyze::utility::{lhs_sup_of_linear_constraint_at, normalize_linear_constraint},
    constraints::RandomAccessibleLinearConstraint,
};

use super::{
    identify_propagation_causals::IdentifyPropagationCausals, round::Round,
    utility::strengthen_integer_linear_constraint, weaken::Weaken,
};

pub struct RoundReasonConstraint {
    integrality_tolerance: f64,
    identify_causals: IdentifyPropagationCausals,
    weaken: Weaken,
    round: Round,
    // linear_constraint: LinearConstraint<u64>,
}

impl RoundReasonConstraint {
    pub fn new(integrality_tolerance: f64) -> Self {
        Self {
            integrality_tolerance,
            identify_causals: IdentifyPropagationCausals::new(),
            weaken: Weaken::new(),
            round: Round::new(integrality_tolerance),
        }
    }

    pub fn round(
        &mut self,
        reason_constraint: &impl LinearConstraintTrait<Value = u64>,
        conflict_constraint: &RandomAccessibleLinearConstraint<u64>,
        propagated_assignment: Literal,
        pb_engine: &PBEngine,
    ) {
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
                (
                    // conflict_constraint に含まれるリテラルを優先
                    if conflict_constraint.get(!literal).is_some() {
                        2
                    } else if conflict_constraint.get(literal).is_some() {
                        1
                    } else {
                        0
                    },
                    // 割り当て順が早いリテラルを優先
                    usize::MAX - pb_engine.get_assignment_order(literal.index()), // activity が大きいリテラルを優先
                                                                                  // OrderedFloat::from(pb_engine.activity(literal.index()))
                )
            },
            pb_engine,
        );
        debug_assert!(
            causal_assignments
                .iter()
                .all(|literal| pb_engine.is_true(literal))
        );

        // 僅かな効果がありそうだが，微妙すぎてよくわからないので一旦コメントアウト
        // あとで再度検討

        // weaken
        // let weakened_reason_constraint = self.weaken.call(
        //     reason_constraint,
        //     move |literal| {
        //         if literal == propagated_assignment || causal_assignments.contains_key(!literal) {
        //             None
        //         } else {
        //             Some(0)
        //         }
        //     },
        //     pb_engine,
        // );
        // #[cfg(debug_assertions)]
        // {
        //     let propagated_coefficient = weakened_reason_constraint
        //         .iter_terms()
        //         .find(|&(literal, _)| literal == propagated_assignment)
        //         .unwrap()
        //         .1;
        //     let sup_at_propagted = weakened_reason_constraint
        //         .iter_terms()
        //         .filter(|&(literal, _)| !causal_assignments.contains_key(!literal))
        //         .map(|(_, coefficient)| coefficient)
        //         .sum::<u64>();
        //     debug_assert!(sup_at_propagted >= weakened_reason_constraint.lower());
        //     debug_assert!(
        //         sup_at_propagted < weakened_reason_constraint.lower() + propagated_coefficient
        //     );
        // }

        // TODO 検討
        // multiple: LinearConstraint<u64> x u64 -> LinearConstraint<u128>
        // rounding: LinearConstraint<u128> x u128 -> LinearConstraint<u128>
        // とすれば浮動小数点数を使わない実装が可能な気がする

        // let strengthened_wakened_reason_constraint = strengthen_integer_linear_constraint(&weakened_reason_constraint);

        // normalize
        // let normalized_reason_constraint =
        // normalize_linear_constraint(&strengthened_wakened_reason_constraint, propagated_assignment);
        let normalized_reason_constraint =
            normalize_linear_constraint(&reason_constraint, propagated_assignment);

        #[cfg(debug_assertions)]
        {
            debug_assert!(
                normalized_reason_constraint
                    .iter_terms()
                    .find(|&(literal, _)| literal == propagated_assignment)
                    .unwrap()
                    .1
                    == 1.0
            );
            let sup_at_propaged = lhs_sup_of_linear_constraint_at(
                &normalized_reason_constraint,
                pb_engine.get_assignment_order(propagated_assignment.index()),
                pb_engine,
            );
            debug_assert!(
                sup_at_propaged
                    < normalized_reason_constraint.lower() + 1.0 - self.integrality_tolerance,
                "{} {}",
                sup_at_propaged,
                normalized_reason_constraint.lower()
            );
        }

        let multipler = conflict_constraint.get(!propagated_assignment).unwrap();

        // round
        self.round.calculate(
            &normalized_reason_constraint,
            |literal| causal_assignments.contains_key(!literal),
            move |literal| {
                conflict_constraint
                    .get(!literal)
                    .map(|anticoefficient| anticoefficient as f64 / multipler as f64)
                    .unwrap_or(0.0)
            },
            pb_engine,
        );
        let rounded_reason_constraint = self.round.get();
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

        // self.linear_constraint.replace(&rounded_reason_constraint);
        // return &self.linear_constraint;
    }

    pub fn get(&self) -> impl LinearConstraintTrait<Value = u64> {
        self.round.get()
    }
}
