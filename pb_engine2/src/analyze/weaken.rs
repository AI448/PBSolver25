use std::{cmp::min, fmt::Debug, iter::Sum, ops::SubAssign};

use num::{PrimInt, Unsigned};

use crate::{ pb_engine::{LinearConstraintTrait, PBEngine}, Literal};

struct CausalTerm<ValueT> {
    literal: Literal,
    coefficient: ValueT,
    probability: f64,
}

struct WeakeningTerm<ValueT> {
    literal: Literal,
    coefficient: ValueT,
    coefficient_lower: ValueT,
    probability: f64,
}

struct Work<ValueT> {
    fixed_terms: Vec<CausalTerm<ValueT>>,
    weakening_terms: Vec<WeakeningTerm<ValueT>>,
}

impl<ValueT> Default for Work<ValueT> {
    fn default() -> Self {
        Self {
            fixed_terms: Vec::default(),
            weakening_terms: Vec::default(),
        }
    }
}

impl<ValueT> Clone for Work<ValueT> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

#[derive(Clone)]
pub struct Weaken<ValueT> {
    work: Work<ValueT>,
}

impl<ValueT> Weaken<ValueT>
where
    ValueT: PrimInt + Unsigned + SubAssign + Sum + Debug,
{
    pub fn new() -> Self {
        Self {
            work: Work::default(),
        }
    }

    pub fn call(
        &mut self,
        constraint: impl LinearConstraintTrait<Value = ValueT>,
        can_reduce_to: impl Fn(Literal) -> Option<ValueT>,
        pb_engine: &PBEngine<u64>,
    ) -> impl LinearConstraintTrait<Value = ValueT> + '_ {
        let work = &mut self.work;

        work.fixed_terms.clear();
        work.weakening_terms.clear();

        // weaken する前の左辺値-右辺値の期待値
        let mut initial_expectation = -constraint.lower().to_f64().unwrap();
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
            initial_expectation += coefficient.to_f64().unwrap() * probability;
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
                expectation += reduction.to_f64().unwrap()
                    * (1.0 - term.probability - sum_of_probability_of_saturatings);
                while l < work.fixed_terms.len() {
                    let falsified_term = &work.fixed_terms[l];
                    if falsified_term.coefficient > lower {
                        expectation -= (falsified_term.coefficient - lower).to_f64().unwrap()
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

        let lower: ValueT = constraint.lower()
            - work.weakening_terms[..best_k]
                .iter()
                .map(|term| term.coefficient - term.coefficient_lower)
                .sum();

        let weakened_constraint = LinearConstraintView::new(
            work.fixed_terms
                .iter()
                .map(move |term| (term.literal, min(term.coefficient, lower)))
                .chain(
                    work.weakening_terms[..best_k]
                        .iter()
                        .filter(|term| term.coefficient_lower != ValueT::zero())
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
