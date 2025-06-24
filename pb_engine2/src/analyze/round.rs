use std::cmp::min;

use num::{Num, PrimInt, Unsigned};

use crate::{
    Literal,
    constraint::{ConstraintView, LinearConstraintTrait, UnsignedIntegerTrait},
    pb_engine::PBEngine,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rounding {
    Integer,
    Up,
    Down,
}

pub struct Round2<ValueT> {
    work: Work2<ValueT>,
}

impl<ValueT> Clone for Round2<ValueT> {
    fn clone(&self) -> Self {
        Self {
            work: Work2::default(),
        }
    }
}

impl<ValueT> Round2<ValueT>
where
    ValueT: UnsignedIntegerTrait,
{
    pub fn new() -> Self {
        Self {
            work: Work2::default(),
        }
    }

    pub fn calculate(
        &mut self,
        constraint: &impl LinearConstraintTrait<Value = ValueT>,
        divisor: ValueT,
        is_causal: impl Fn(Literal) -> bool,
        engine: &PBEngine,
    ) -> impl LinearConstraintTrait<Value = ValueT> + '_ {
        let work = &mut self.work;

        work.terms.clear();
        let mut lower = constraint.lower();
        for (literal, coefficient) in constraint.iter_terms() {
            let rounding;
            let switching_priority;
            if (coefficient % divisor).is_zero() {
                // 整数の場合
                rounding = Rounding::Integer;
                switching_priority = 0.0;
            } else if is_causal(literal) {
                // 切り上げる
                rounding = Rounding::Up;
                switching_priority = (1.0 - engine.assignment_probability(!literal))
                    / ((coefficient % divisor).to_f64().unwrap() / divisor.to_f64().unwrap());
            } else {
                // 切り下げる
                rounding = Rounding::Down;
                switching_priority = engine.assignment_probability(!literal)
                    / (1.0 - (coefficient % divisor).to_f64().unwrap() / divisor.to_f64().unwrap());
                lower = lower - coefficient % divisor;
            }
            work.terms.push(Term2 {
                literal,
                coefficient,
                rounding,
                switching_priority,
            });
        }

        // 丸め方向切り替えの優先度が高い順にソート
        work.terms.sort_unstable_by(|l, r| {
            r.switching_priority.partial_cmp(&l.switching_priority).unwrap()
        });

        for term in work.terms.iter_mut() {
            match term.rounding {
                Rounding::Up => {
                    let d = term.coefficient % divisor;
                    if (lower + divisor - ValueT::one()) / divisor
                        == (lower + divisor - d - ValueT::one()) / divisor
                    {
                        // 切り上げていた係数を切り下げる
                        term.rounding = Rounding::Down;
                        lower = lower - d;
                    }
                }
                Rounding::Down => {
                    let d = divisor - term.coefficient % term.coefficient;
                    if (lower + divisor - ValueT::one()) / divisor
                        == (lower + divisor - d - ValueT::one()) / divisor
                    {
                        term.rounding = Rounding::Up;
                        lower = lower + divisor - d;
                    }
                }
                _ => {}
            }
        }

        let rounded_constraint = ConstraintView::new(
            self.work.terms.iter().filter_map(move |term| {
                let coefficient = min(
                    match term.rounding {
                        Rounding::Integer => term.coefficient / divisor,
                        Rounding::Up => term.coefficient.div_ceil(&divisor), // (term.coefficient + divisor - ValueT::one()) / divisor,
                        Rounding::Down => term.coefficient / divisor,
                    },
                    lower,
                );
                if !coefficient.is_zero() {
                    Some((term.literal, coefficient))
                } else {
                    None
                }
            }),
            (lower + divisor - ValueT::one()) / divisor,
        );

        return rounded_constraint;
    }
}

struct Term2<ValueT> {
    literal: Literal,
    coefficient: ValueT,
    rounding: Rounding,
    switching_priority: f64,
}

struct Work2<ValueT> {
    terms: Vec<Term2<ValueT>>,
}

impl<ValueT> Default for Work2<ValueT> {
    fn default() -> Self {
        Self {
            terms: Vec::default(),
        }
    }
}

impl<ValueT> Clone for Work2<ValueT>
where
    ValueT: Num + Unsigned + PrimInt,
{
    fn clone(&self) -> Self {
        Self::default()
    }
}
