use either::Either;
use num::{Integer, Num, PrimInt, Unsigned, integer::gcd};
use std::ops::AddAssign;
use std::{cmp::min, fmt::Debug};
use utility::{down_heap, up_heap};

use crate::{LinearConstraintTrait, LinearConstraintView, Literal, PBEngine};

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

pub fn divide_linear_constraint<ValueT>(
    constraint: &impl LinearConstraintTrait<Value = ValueT>,
    divisor: f64,
) -> impl LinearConstraintTrait<Value = f64> + '_
where
    ValueT: PrimInt + Unsigned + Debug,
{
    return LinearConstraintView::new(
        constraint
            .iter_terms()
            .map(move |(literal, coefficient)| (literal, coefficient.to_f64().unwrap() / divisor)),
        constraint.lower().to_f64().unwrap() / divisor,
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

#[derive(Default)]
pub struct StrengthenLinearConstraint {
    terms: Vec<(Literal, u128)>,
    lower: u128,
    work: Vec<(Literal, u128)>,
}

impl StrengthenLinearConstraint {
    pub fn strengthen(
        &mut self,
        constraint: &impl LinearConstraintTrait<Value = u128>,
    ) -> impl LinearConstraintTrait<Value = u128> {
        let compare = |lhs: &(Literal, u128), rhs: &(Literal, u128)| rhs.1.cmp(&lhs.1);

        self.lower = constraint.lower();
        self.terms.clear();
        self.terms.extend(
            constraint
                .iter_terms()
                .map(|(literal, coefficient)| (literal, min(coefficient, self.lower))),
        );

        self.work.clear();
        for &(literal, coefficient) in self.terms.iter() {
            let p = self.work.len();
            self.work.push((literal, coefficient));
            up_heap(&mut self.work, p, compare);
        }
        let mut m = 1;

        loop {
            let sum_of_remainders = self
                .terms
                .iter()
                .map(|&(_, coefficient)| (coefficient as u128 * m as u128) % self.lower as u128)
                .sum::<u128>();
            if sum_of_remainders < self.lower as u128 {
                // let dump = m != self.lower && m != 1 && sum_of_remainders != 0;
                // if dump {
                //     eprintln!("BEFORE {} {}", m, self.lower);
                //     for &(literal, coefficient) in self.terms.iter() {
                //         eprint!("+ {} {} ", coefficient, literal);
                //     }
                //     eprintln!(">= {}", self.lower);
                // }

                for (_, coefficient) in self.terms.iter_mut() {
                    *coefficient = *coefficient * m / self.lower;
                }
                self.terms.retain(|&(_, coefficient)| coefficient != 0);
                self.lower = m;

                // if dump {
                //     eprintln!("AFTER");
                //     for &(literal, coefficient) in self.terms.iter() {
                //         eprint!("+ {} {} ", coefficient, literal);
                //     }
                //     eprintln!(">= {}", self.lower);
                // }

                if self.terms.is_empty() || self.lower == m || self.lower == 1 {
                    return LinearConstraintView::new(self.terms.iter().cloned(), self.lower);
                }

                self.work.clear();
                for &(literal, coefficient) in self.terms.iter() {
                    let p = self.work.len();
                    self.work.push((literal, coefficient));
                    up_heap(&mut self.work, p, compare);
                }
                m = 1;
            }

            loop {
                let largest = self.work[0].1;
                let new_m = (self.lower + largest / 2) / largest;
                if new_m > m {
                    m = new_m;
                    break;
                }
                let second = self.work[1..]
                    .iter()
                    .take(2)
                    .max_by_key(|&(_, coefficient)| coefficient);
                if second.is_none_or(|&(_, coefficient)| coefficient == 0) {
                    for (_, coefficient) in self.terms.iter_mut() {
                        debug_assert!(*coefficient % largest == 0);
                        *coefficient /= largest;
                    }
                    self.lower = self.lower.div_ceil(largest);
                    return LinearConstraintView::new(self.terms.iter().cloned(), self.lower);
                }
                debug_assert!(self.work[0].1 >= second.unwrap().1);
                self.work[0].1 %= second.unwrap().1;
                down_heap(&mut self.work, 0, compare);
            }
        }
    }

    // pub fn strengthen(
    //     &mut self,
    //     constraint: &impl LinearConstraintTrait<Value = u128>
    // ) -> impl LinearConstraintTrait<Value = u128> {

    //     let compare = |lhs: &(Literal, u128), rhs: &(Literal, u128)| rhs.1.cmp(&lhs.1);

    //     self.lower = constraint.lower();
    //     self.terms.clear();
    //     self.terms.extend(constraint.iter_terms().map(|(literal, coefficient)| (literal, min(coefficient, self.lower))));

    //     self.work.clear();
    //     for &(literal, coefficient) in self.terms.iter() {
    //         let p = self.work.len();
    //         self.work.push((literal, coefficient));
    //         up_heap(&mut self.work, p, compare);
    //     }

    //     let mut m = 1;
    //     loop {
    //         let sum_of_remainders = self.terms.iter().map(|&(_, coefficient)| (coefficient as u128 * m as u128) % self.lower as u128).sum::<u128>();
    //         if sum_of_remainders < self.lower as u128 {
    //             self.work.clear();
    //             self.work.extend(self.terms.iter().map(|&(literal, coefficient)| (literal, (coefficient as u128 * m as u128 / self.lower as u128) as u128)).filter(|&(_, coefficient)| coefficient != 0));
    //             if m != self.lower && m != 1 && sum_of_remainders != 0 {
    //                 eprintln!("BEFORE");
    //                 for &(literal, coefficient) in self.terms.iter() {
    //                     eprint!("+ {} {} ", coefficient, literal);
    //                 }
    //                 eprintln!(">= {}", self.lower);
    //                 eprintln!("AFTER");
    //                 for &(literal, coefficient) in self.work.iter() {
    //                     eprint!("+ {} {} ", coefficient, literal);
    //                 }
    //                 eprintln!(">= {}", m);
    //             }
    //             return LinearConstraintView::new(
    //                 self.work.iter().cloned(),
    //                 m
    //             );
    //         }
    //         loop {
    //             let largest = self.work[0].1;
    //             let new_m = (self.lower + largest / 2) / largest;
    //             if new_m > m {
    //                 m = new_m;
    //                 break;
    //             }
    //             let second = self.work[1..].iter().take(2).max_by_key(|&(_, coefficient)| coefficient);
    //             if second.is_none_or(|&(_, coefficient)| coefficient == 0) {
    //                 self.work.clear();
    //                 self.work.extend(self.terms.iter().map(|&(literal, coefficient)| (literal, coefficient / largest)));
    //                 return LinearConstraintView::new(
    //                     self.work.iter().cloned(),
    //                     self.lower.div_ceil(largest)
    //                 );
    //             }
    //             debug_assert!(self.work[0].1 >= second.unwrap().1);
    //             self.work[0].1 %= second.unwrap().1;
    //             down_heap(&mut self.work, 0, compare);
    //         }
    //     }

    // }
}

// pub fn strengthen_integer_linear_constraint<ValueT>(
//     constraint: &impl LinearConstraintTrait<Value = ValueT>,
// ) -> impl LinearConstraintTrait<Value = ValueT>
// where
//     ValueT: Integer + PrimInt + Unsigned + AddAssign + Debug,
// {
//     let lower = constraint.lower();

//     let mut sum_of_unsaturating_coefficients = ValueT::zero();
//     for (_, coefficient) in constraint.iter_terms() {
//         if coefficient < lower {
//             sum_of_unsaturating_coefficients += coefficient;
//         }
//     }

//     if sum_of_unsaturating_coefficients < lower {
//         return Either::Left(LinearConstraintView::new(
//             constraint
//                 .iter_terms()
//                 .filter(move |&(_, coefficient)| coefficient >= lower),
//             ValueT::one(),
//         ));
//     } else {
//         let gcd = calculate_gcd(
//             constraint
//                 .iter_terms()
//                 .map(|(_, coefficient)| min(coefficient, lower)),
//         );
//         return Either::Right(LinearConstraintView::new(
//             constraint
//                 .iter_terms()
//                 .filter_map(move |(literal, coefficient)| {
//                     if coefficient != ValueT::zero() {
//                         debug_assert!(min(coefficient, lower) % gcd == ValueT::zero());
//                         Some((literal, min(coefficient, lower) / gcd))
//                     } else {
//                         None
//                     }
//                 }),
//             constraint.lower().div_ceil(&gcd),
//         ));
//     }
// }

pub fn calculate_gcd<ValueT>(values: impl Iterator<Item = ValueT>) -> ValueT
where
    ValueT: Integer,
{
    let mut x = ValueT::zero();
    for y in values {
        if x.is_one() {
            break;
        }
        x = gcd(x, y);
    }
    return x;
}

// pub fn gcd<ValueT>(mut x: ValueT, mut y: ValueT) -> ValueT
// where
//     ValueT: Unsigned + Ord + Copy,
// {
//     if x > y {
//         let z = x;
//         x = y;
//         y = z;
//     }

//     loop {
//         debug_assert!(x <= y);
//         if x.is_zero() {
//             return y;
//         }
//         let z = y % x;
//         y = x;
//         x = z;
//     }
// }
