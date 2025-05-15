use std::cmp::min;

use crate::{LinearConstraintTrait, LinearConstraintView, Literal, PBEngine};

struct CausalTerm {
    literal: Literal,
    coefficient: u64,
    probability: f64,
}

struct WeakeningTerm {
    literal: Literal,
    coefficient: u64,
    coefficient_lower: u64,
    probability: f64,
}

#[derive(Default)]
struct Work {
    fixed_terms: Vec<CausalTerm>,
    weakening_terms: Vec<WeakeningTerm>,
}

impl Clone for Work {
    fn clone(&self) -> Self {
        Self::default()
    }
}

#[derive(Clone)]
pub struct Weaken {
    work: Work,
}

impl Weaken {
    pub fn new() -> Self {
        Self {
            work: Work::default(),
        }
    }

    pub fn call<'a>(
        &'a mut self,
        constraint: &impl LinearConstraintTrait<Value = u64>,
        can_reduce_to: impl Fn(Literal) -> Option<u64>,
        pb_engine: &PBEngine,
    ) -> impl LinearConstraintTrait<Value = u64> + 'a {
        let work = &mut self.work;

        work.fixed_terms.clear();
        work.weakening_terms.clear();

        // weaken する前の左辺値-右辺値の期待値
        let mut initial_expectation = -(constraint.lower() as f64);
        for (literal, coefficient) in constraint.iter_terms() {
            let probability = 1.0 - pb_engine.assignment_probability(!literal);
            if let Some(coefficient_lower) = can_reduce_to(literal)
                && coefficient_lower < coefficient
            {
                work.weakening_terms.push(WeakeningTerm {
                    literal,
                    coefficient,
                    coefficient_lower,
                    probability,
                });
            } else {
                work.fixed_terms.push(CausalTerm {
                    literal,
                    coefficient,
                    probability,
                });
            }
            initial_expectation += coefficient as f64 * probability;
        }

        // 係数が大きい順にソート
        work.fixed_terms
            .sort_unstable_by(|l, r| r.coefficient.partial_cmp(&l.coefficient).unwrap());

        // False が割り当てられない確率が高い順にソート
        work.weakening_terms
            .sort_unstable_by(|l, r| r.probability.partial_cmp(&l.probability).unwrap());

        // 何番目までの項を weaken すれば期待値が最小になるか
        let mut best_k = 0;
        {
            let mut lower = constraint.lower();
            let mut min_expectation = initial_expectation;
            let mut expectation = initial_expectation;
            let mut sum_of_probability_of_saturatings = 0.0;
            let mut l = 0;
            for k in 1..=work.weakening_terms.len() {
                let term = &work.weakening_terms[k - 1];
                // term のリテラルを True に固定したと仮定して制約条件の下限と sup - lower の期待値を更新
                let reduction = term.coefficient - term.coefficient_lower;
                lower -= reduction;
                expectation +=
                    reduction as f64 * (1.0 - term.probability - sum_of_probability_of_saturatings);
                while l < work.fixed_terms.len() {
                    let falsified_term = &work.fixed_terms[l];
                    if falsified_term.coefficient > lower {
                        expectation -= (falsified_term.coefficient - lower) as f64
                            * falsified_term.probability;
                        sum_of_probability_of_saturatings += falsified_term.probability;
                        l += 1;
                    } else {
                        break;
                    }
                }
                if expectation < min_expectation {
                    min_expectation = expectation;
                    best_k = k;
                }
            }
        }

        let lower = constraint.lower()
            - work.weakening_terms[..best_k]
                .iter()
                .map(|term| term.coefficient - term.coefficient_lower)
                .sum::<u64>();

        let weakened_constraint = LinearConstraintView::new(
            work.fixed_terms
                .iter()
                .map(move |term| (term.literal, min(term.coefficient, lower)))
                .chain(
                    work.weakening_terms[..best_k]
                        .iter()
                        .filter(|term| term.coefficient_lower != 0)
                        .map(move |term| (term.literal, min(term.coefficient_lower, lower))),
                )
                .chain(
                    work.weakening_terms[best_k..]
                        .iter()
                        .map(move |term| (term.literal, min(term.coefficient, lower))),
                ),
            lower,
        );

        return weakened_constraint;
    }
}
