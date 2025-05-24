use std::{cmp::min, fmt::Debug};

use num::{Integer, Num, PrimInt, Signed, Unsigned};

use crate::{LinearConstraintTrait, LinearConstraintView, Literal, PBEngine};

#[derive(Clone)]
pub struct Round {
    integrality_tolerance: f64,
    work: Work,
}

impl Round {
    pub fn new(integrality_tolerance: f64) -> Self {
        Self {
            integrality_tolerance,
            work: Work::default(),
        }
    }

    pub fn calculate(
        &mut self,
        constraint: &impl LinearConstraintTrait<Value = f64>,
        is_causal: impl Fn(Literal) -> bool,
        get_anticoefficient: impl Fn(Literal) -> f64,
        engine: &PBEngine,
    ) {
        let work = &mut self.work;

        work.terms.clear();
        let mut lower = constraint.lower();
        for (literal, coefficient) in constraint.iter_terms() {
            let rounding;
            let switching_priority;
            if (coefficient - coefficient.round()).abs() <= self.integrality_tolerance {
                // 整数の場合
                rounding = Rounding::Integer;
                switching_priority = 0.0;
                if coefficient > coefficient.round() {
                    lower -= coefficient - coefficient.round();
                }
            } else if is_causal(literal) {
                // 切り上げる
                rounding = Rounding::Up;
                switching_priority = {
                    let anticoefficient = get_anticoefficient(literal);
                    // 切り上げた場合の期待値
                    let before_experiment = if coefficient.ceil() > anticoefficient {
                        (coefficient.ceil() - anticoefficient)
                            * (1.0 - engine.assignment_probability(!literal))
                    } else {
                        (anticoefficient - coefficient.ceil())
                            * (1.0 - engine.assignment_probability(literal))
                    } + f64::min(coefficient.ceil(), anticoefficient);
                    // 切り下げた場合の期待値
                    let after_experiment = if coefficient.floor() > anticoefficient {
                        (coefficient.floor() - anticoefficient)
                            * (1.0 - engine.assignment_probability(!literal))
                    } else {
                        (anticoefficient - coefficient.floor())
                            * (1.0 - engine.assignment_probability(literal))
                    } + f64::min(coefficient.floor(), anticoefficient);
                    // 期待値の減少量を priority とする
                    before_experiment - after_experiment
                };
            } else {
                // 切り下げる
                rounding = Rounding::Down;
                switching_priority = {
                    let anticoefficient = get_anticoefficient(literal);
                    // 切り下げた場合の期待値
                    let before_experiment = if coefficient.floor() >= anticoefficient {
                        (coefficient.floor() - anticoefficient)
                            * (1.0 - engine.assignment_probability(!literal))
                    } else {
                        (anticoefficient - coefficient.floor())
                            * (1.0 - engine.assignment_probability(literal))
                    } + f64::min(coefficient.floor(), anticoefficient);
                    // 切り上げた場合の期待値
                    let after_experiment = if coefficient.ceil() >= anticoefficient {
                        (coefficient.ceil() - anticoefficient)
                            * (1.0 - engine.assignment_probability(!literal))
                    } else {
                        (anticoefficient - coefficient.ceil())
                            * (1.0 - engine.assignment_probability(literal))
                    } + f64::min(coefficient.ceil(), anticoefficient)
                        - 1.0;
                    // 期待値の減少量を priority とする
                    before_experiment - after_experiment
                };
                lower -= coefficient - coefficient.floor();
            }
            work.terms.push(Term {
                literal,
                coefficient,
                rounding,
                switching_priority,
            });
        }

        // 丸め方向切り替えの優先度が高い順にソート
        work.terms.sort_unstable_by(|l, r| {
            r.switching_priority
                .partial_cmp(&l.switching_priority)
                .unwrap()
        });

        for term in work.terms.iter_mut() {
            match term.rounding {
                Rounding::Up => {
                    let d = term.coefficient - term.coefficient.floor();
                    if (lower - self.integrality_tolerance).ceil()
                        == (lower - d - self.integrality_tolerance).ceil()
                    {
                        // 切り上げていた係数を切り下げる
                        term.rounding = Rounding::Down;
                        lower += -d;
                    }
                }
                Rounding::Down => {
                    let d = term.coefficient.ceil() - term.coefficient;
                    if (lower - self.integrality_tolerance).ceil()
                        == (lower - d - self.integrality_tolerance).ceil()
                    {
                        term.rounding = Rounding::Up;
                        lower += -d + 1.0;
                    }
                }
                _ => {}
            }
        }

        work.lower = (lower - self.integrality_tolerance).ceil() as u64;
    }

    pub fn get(&self) -> impl LinearConstraintTrait<Value = u64> {
        let rounded_constraint = LinearConstraintView::new(
            self.work.terms.iter().filter_map(move |term| {
                let coefficient = min(
                    match term.rounding {
                        Rounding::Integer => term.coefficient.round() as u64,
                        Rounding::Up => term.coefficient.ceil() as u64,
                        Rounding::Down => term.coefficient.floor() as u64,
                    },
                    self.work.lower,
                );
                if coefficient != 0 {
                    Some((term.literal, coefficient))
                } else {
                    None
                }
            }),
            self.work.lower,
        );

        return rounded_constraint;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rounding {
    Integer,
    Up,
    Down,
}

struct Term {
    literal: Literal,
    coefficient: f64,
    rounding: Rounding,
    switching_priority: f64,
}

#[derive(Default)]
struct Work {
    terms: Vec<Term>,
    lower: u64,
}

impl Clone for Work {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl LinearConstraintTrait for Work {
    type Value = u64;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone + '_ {
        return self.terms.iter().filter_map(move |term| {
            let coefficient = min(
                match term.rounding {
                    Rounding::Integer => term.coefficient.round() as u64,
                    Rounding::Up => term.coefficient.ceil() as u64,
                    Rounding::Down => term.coefficient.floor() as u64,
                },
                self.lower,
            );
            if coefficient != 0 {
                Some((term.literal, coefficient))
            } else {
                None
            }
        });
    }

    fn lower(&self) -> Self::Value {
        self.lower
    }
}

#[derive(Clone)]
pub struct Round2<ValueT>
where
    ValueT: Integer + PrimInt + Unsigned + Debug,
{
    work: Work2<ValueT>,
}

impl<ValueT> Round2<ValueT>
where
    ValueT: Integer + PrimInt + Unsigned + Debug,
{
    pub fn new() -> Self {
        Self {
            work: Work2::default(),
        }
    }

    pub fn calculate(
        &mut self,
        constraint: impl LinearConstraintTrait<Value = ValueT>,
        divisor: ValueT,
        is_causal: impl Fn(Literal) -> bool,
        get_anticoefficient: impl Fn(Literal) -> f64,
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
                switching_priority = {
                    let anticoefficient = get_anticoefficient(literal);
                    // 切り上げた場合の期待値
                    let ceiled_coefficient = coefficient.div_ceil(&divisor).to_f64().unwrap();
                    let before_experiment = if ceiled_coefficient > anticoefficient {
                        (ceiled_coefficient - anticoefficient)
                            * (1.0 - engine.assignment_probability(!literal))
                    } else {
                        (anticoefficient - ceiled_coefficient)
                            * (1.0 - engine.assignment_probability(literal))
                    } + f64::min(ceiled_coefficient, anticoefficient);
                    // 切り下げた場合の期待値
                    let floored_coefficient = (coefficient / divisor).to_f64().unwrap();
                    let after_experiment = if floored_coefficient > anticoefficient {
                        (floored_coefficient - anticoefficient)
                            * (1.0 - engine.assignment_probability(!literal))
                    } else {
                        (anticoefficient - floored_coefficient)
                            * (1.0 - engine.assignment_probability(literal))
                    } + f64::min(floored_coefficient, anticoefficient);
                    // 期待値の減少量を priority とする
                    before_experiment - after_experiment
                };
            } else {
                // 切り下げる
                rounding = Rounding::Down;
                switching_priority = {
                    let anticoefficient = get_anticoefficient(literal);
                    // 切り下げた場合の期待値
                    let floored_coefficient = (coefficient / divisor).to_f64().unwrap();
                    let before_experiment = if floored_coefficient >= anticoefficient {
                        (floored_coefficient - anticoefficient)
                            * (1.0 - engine.assignment_probability(!literal))
                    } else {
                        (anticoefficient - floored_coefficient)
                            * (1.0 - engine.assignment_probability(literal))
                    } + f64::min(floored_coefficient, anticoefficient);
                    // 切り上げた場合の期待値
                    let ceiled_coefficient = ((coefficient + divisor - ValueT::one()) / divisor)
                        .to_f64()
                        .unwrap();
                    let after_experiment = if ceiled_coefficient >= anticoefficient {
                        (ceiled_coefficient - anticoefficient)
                            * (1.0 - engine.assignment_probability(!literal))
                    } else {
                        (anticoefficient - ceiled_coefficient)
                            * (1.0 - engine.assignment_probability(literal))
                    } + f64::min(ceiled_coefficient, anticoefficient)
                        - 1.0;
                    // 期待値の減少量を priority とする
                    before_experiment - after_experiment
                };
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
            r.switching_priority
                .partial_cmp(&l.switching_priority)
                .unwrap()
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

        let rounded_constraint = LinearConstraintView::new(
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

        // work.lower = (lower - self.integrality_tolerance).ceil() as u64;
    }

    // pub fn get(&self) -> impl LinearConstraintTrait<Value = ValueT> {
    //     let rounded_constraint = LinearConstraintView::new(
    //         self.work.terms.iter().filter_map(move |term| {
    //             let coefficient = min(
    //                 match term.rounding {
    //                     Rounding::Integer => term.coefficient / self.work.divisor,
    //                     Rounding::Up => (term.coefficient + self.work.divisor - ValueT::one()) / self.work.divisor,
    //                     Rounding::Down => term.coefficient / self.work.divisor,
    //                 },
    //                 self.work.lower,
    //             );
    //             if !coefficient.is_zero() {
    //                 Some((term.literal, coefficient))
    //             } else {
    //                 None
    //             }
    //         }),
    //         self.work.lower,
    //     );

    //     return rounded_constraint;
    // }
}

struct Term2<ValueT> {
    literal: Literal,
    coefficient: ValueT,
    rounding: Rounding,
    switching_priority: f64,
}

struct Work2<ValueT>
where
    ValueT: Num + Unsigned + PrimInt,
{
    terms: Vec<Term2<ValueT>>,
    // lower: ValueT,
    // divisor: ValueT,
}

impl<ValueT> Default for Work2<ValueT>
where
    ValueT: Num + Unsigned + PrimInt,
{
    fn default() -> Self {
        Self {
            terms: Vec::default(), // , lower: ValueT::zero(), divisor: ValueT::zero()
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

// impl<ValueT> LinearConstraintTrait for Work2<ValueT>
// where
//     ValueT: Num + Unsigned + PrimInt + Debug
// {
//     type Value = ValueT;
//     fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone + '_ {
//         return self.terms.iter().filter_map(move |term| {
//             let coefficient = min(
//                 match term.rounding {
//                     Rounding::Integer => term.coefficient,
//                     Rounding::Up => (term.coefficient + self.divisor - ValueT::one()) / self.divisor,
//                     Rounding::Down => term.coefficient / self.divisor * self.divisor,
//                 },
//                 self.lower,
//             );
//             if !coefficient.is_zero() {
//                 Some((term.literal, coefficient))
//             } else {
//                 None
//             }
//         });
//     }

//     fn lower(&self) -> Self::Value {
//         self.lower
//     }
// }
