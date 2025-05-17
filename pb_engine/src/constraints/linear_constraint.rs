use std::{
    fmt::Debug,
    ops::{AddAssign, Mul, SubAssign},
};

use either::Either;
use num::Num;
use utility::Map;

use crate::{Boolean, Literal};

// 値の型をジェネリックパラメータとして，整数と浮動小数点数とで Constraint の実装を統合できないか
// ※ Theory の実装は tolerance と，数値誤差の蓄積を考慮する必要があるので実装を分けざるを得ない
pub trait LinearConstraintTrait {
    type Value: Num + Copy + Debug;

    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone + '_;

    fn lower(&self) -> Self::Value;

    fn len(&self) -> usize {
        self.iter_terms().count()
    }

    fn mul(&self, multipler: Self::Value) -> impl LinearConstraintTrait<Value = Self::Value>
    where
        Self::Value: AddAssign + Mul,
    {
        return LinearConstraintView::new(
            self.iter_terms()
                .map(move |(literal, coefficient)| (literal, coefficient * multipler)),
            self.lower() * multipler,
        );
    }
}

impl<LinearConstraintT> LinearConstraintTrait for &LinearConstraintT
where
    LinearConstraintT: LinearConstraintTrait,
{
    type Value = LinearConstraintT::Value;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone + '_ {
        (*self).iter_terms()
    }

    fn lower(&self) -> Self::Value {
        (*self).lower()
    }
}

impl<LhsLinearConstraintT, RhsLinearConstraintT, ValueT> LinearConstraintTrait
    for Either<LhsLinearConstraintT, RhsLinearConstraintT>
where
    LhsLinearConstraintT: LinearConstraintTrait<Value = ValueT>,
    RhsLinearConstraintT: LinearConstraintTrait<Value = ValueT>,
    ValueT: Num + Copy + Debug,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone + '_ {
        return match self {
            Either::Left(left) => Either::Left(left.iter_terms()),
            Either::Right(right) => Either::Right(right.iter_terms()),
        };
    }

    fn lower(&self) -> Self::Value {
        return match self {
            Either::Left(left) => left.lower(),
            Either::Right(right) => right.lower(),
        };
    }
}

#[derive(Default, Clone, Debug)]
pub struct LinearConstraint<ValueT>
where
    ValueT: Copy + Debug,
{
    terms: Vec<(Literal, ValueT)>,
    lower: ValueT,
}

impl<ValueT> LinearConstraint<ValueT>
where
    ValueT: Copy + Debug,
{
    pub fn new(terms: impl Iterator<Item = (Literal, ValueT)>, lower: ValueT) -> Self {
        Self {
            terms: Vec::from_iter(terms),
            lower: lower,
        }
    }

    pub fn replace(&mut self, linear_constraint: impl LinearConstraintTrait<Value = ValueT>) {
        self.terms.clear();
        self.terms.extend(linear_constraint.iter_terms());
        self.lower = linear_constraint.lower();
    }
}

impl<ValueT> LinearConstraintTrait for LinearConstraint<ValueT>
where
    ValueT: Num + Copy + Debug,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone + '_ {
        self.terms.iter().cloned()
    }

    fn lower(&self) -> Self::Value {
        self.lower
    }
}

pub struct LinearConstraintView<ValueT, IteratorT>
where
    IteratorT: Iterator<Item = (Literal, ValueT)> + Clone,
{
    iterator: IteratorT,
    lower: ValueT,
}

impl<ValueT, IteratorT> LinearConstraintView<ValueT, IteratorT>
where
    IteratorT: Iterator<Item = (Literal, ValueT)> + Clone,
{
    pub fn new(iterator: IteratorT, lower: ValueT) -> Self {
        Self { iterator, lower }
    }
}

impl<ValueT, IteratorT> LinearConstraintTrait for LinearConstraintView<ValueT, IteratorT>
where
    ValueT: Num + Copy + Debug,
    IteratorT: Iterator<Item = (Literal, ValueT)> + Clone,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone + '_ {
        self.iterator.clone()
    }

    fn lower(&self) -> Self::Value {
        self.lower
    }
}

#[derive(Default, Clone)]
pub struct RandomAccessibleLinearConstraint<ValueT>
where
    ValueT: Copy,
{
    terms: Map<(Boolean, ValueT)>,
    lower: ValueT,
}

impl<ValueT> LinearConstraintTrait for RandomAccessibleLinearConstraint<ValueT>
where
    ValueT: Num + Copy + Debug,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone + '_ {
        return self
            .terms
            .iter()
            .map(|(&index, term)| (Literal::new(index, term.0), term.1.clone()));
    }

    fn lower(&self) -> Self::Value {
        return self.lower;
    }
}

impl<ValueT> RandomAccessibleLinearConstraint<ValueT>
where
    ValueT: Num + AddAssign + SubAssign + PartialOrd + Copy + Debug,
{
    pub fn get(&self, literal: Literal) -> Option<ValueT> {
        return self
            .terms
            .get(literal.index())
            .filter(|term| term.0 == literal.value())
            .map(|term| term.1.clone());
    }

    pub fn get_mut(&mut self, literal: Literal) -> Option<&mut ValueT> {
        return self
            .terms
            .get_mut(literal.index())
            .filter(|term| term.0 == literal.value())
            .map(|term| &mut term.1);
    }

    pub fn replace_by_linear_constraint(
        &mut self,
        linear_constraint: &impl LinearConstraintTrait<Value = ValueT>,
    ) {
        self.terms.clear();
        self.terms.extend(
            linear_constraint
                .iter_terms()
                .filter(|(_, coefficient)| *coefficient != ValueT::zero())
                .map(|(literal, coefficient)| (literal.index(), (literal.value(), coefficient))),
        );
        self.lower = linear_constraint.lower();
    }

    pub fn iter_terms_mut(&mut self) -> impl Iterator<Item = (Literal, &mut ValueT)> + '_ {
        return self
            .terms
            .iter_mut()
            .map(|(&index, term)| (Literal::new(index, term.0), &mut term.1));
    }

    pub fn lower_mut(&mut self) -> &mut ValueT {
        &mut self.lower
    }

    pub fn add_assign(&mut self, reason_constraint: &impl LinearConstraintTrait<Value = ValueT>) {
        self.lower += reason_constraint.lower();
        for (literal, coefficient) in reason_constraint.iter_terms() {
            if let Some(term) = self.terms.get_mut(literal.index()) {
                if term.0 == literal.value() {
                    term.1 += coefficient;
                } else {
                    if term.1 > coefficient {
                        self.lower -= coefficient;
                        term.1 -= coefficient;
                    } else if term.1 < coefficient {
                        self.lower -= term.1;
                        term.0 = !term.0;
                        term.1 = coefficient - term.1;
                    } else {
                        self.lower -= term.1;
                        self.terms.remove(literal.index());
                    }
                }
            } else {
                self.terms
                    .insert(literal.index(), (literal.value(), coefficient));
            }
        }
    }

    // fn strengthen(&mut self) {
    //     // lower を超える係数を lower まで減少
    //     for (_, term) in self.terms.iter_mut() {
    //         if term.coefficient > self.lower {
    //             term.coefficient = self.lower;
    //         }
    //     }

    //     // lower 未満の係数の合計が lower 未満であればそれらの係数を削除
    //     let sum_of_unsaturatings = self
    //         .terms
    //         .iter()
    //         .map(|(_, term)| term.coefficient)
    //         .filter(|&coefficient| coefficient < self.lower - violation_tolerance)
    //         .sum::<f64>();
    //     if sum_of_unsaturatings < self.lower - violation_tolerance {
    //         self.terms
    //             .retain(|_, term| term.coefficient >= self.lower - violation_tolerance);
    //     }
    // }
}
