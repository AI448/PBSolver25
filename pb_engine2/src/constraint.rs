use crate::{Boolean, Literal, pb_engine::DecisionStack};
use either::Either;
use std::{
    fmt::Display,
    ops::{AddAssign, SubAssign},
};
use utility::Map;

pub trait UnsignedIntegerTrait:
    num::Integer
    + num::Unsigned
    + num::FromPrimitive
    + num::NumCast
    + num::Zero
    + num::One
    + Copy
    + AddAssign
    + SubAssign
    + Display
{
}

impl<T> UnsignedIntegerTrait for T where
    T: num::Integer
        + num::Unsigned
        + num::FromPrimitive
        + num::NumCast
        + num::Zero
        + num::One
        + Copy
        + AddAssign
        + SubAssign
        + Display
{
}

pub trait LinearConstraintTrait {
    type Value: UnsignedIntegerTrait;

    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone;

    fn lower(&self) -> Self::Value;

    fn mul(&self, multipler: Self::Value) -> impl LinearConstraintTrait<Value = Self::Value> {
        return ConstraintView::new(
            self.iter_terms().map(move |(literal, coefficient)| (literal, coefficient * multipler)),
            self.lower() * multipler,
        );
    }

    fn as_view(&self) -> impl LinearConstraintTrait<Value = Self::Value> {
        return ConstraintView::new(self.iter_terms(), self.lower());
    }

    fn convert<ValueT>(&self) -> impl LinearConstraintTrait<Value = ValueT>
    where
        ValueT: UnsignedIntegerTrait,
    {
        return ConstraintView::new(
            self.iter_terms()
                .map(|(literal, coefficient)| (literal, ValueT::from(coefficient).unwrap())),
            ValueT::from(self.lower()).unwrap(),
        );
    }
}

impl<ConstraintT> LinearConstraintTrait for &ConstraintT
where
    ConstraintT: LinearConstraintTrait,
{
    type Value = ConstraintT::Value;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        (*self).iter_terms()
    }

    fn lower(&self) -> Self::Value {
        (*self).lower()
    }
}

impl<LeftT, RightT> LinearConstraintTrait for Either<LeftT, RightT>
where
    LeftT: LinearConstraintTrait,
    RightT: LinearConstraintTrait<Value = LeftT::Value>,
{
    type Value = LeftT::Value;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
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

#[derive(Clone, Debug)]
pub struct LinearConstraint<ValueT> {
    terms: Vec<(Literal, ValueT)>,
    lower: ValueT,
}

impl<ValueT> Default for LinearConstraint<ValueT>
where
    ValueT: UnsignedIntegerTrait,
{
    fn default() -> Self {
        Self {
            terms: Vec::default(),
            lower: ValueT::zero(),
        }
    }
}

impl<ValueT> LinearConstraint<ValueT> {
    pub fn replace(&mut self, terms: impl Iterator<Item = (Literal, ValueT)>, lower: ValueT) {
        self.terms.clear();
        self.terms.extend(terms);
        self.lower = lower;
    }

    pub fn replace_by_linear_constraint(
        &mut self,
        constraint: &impl LinearConstraintTrait<Value = ValueT>,
    ) {
        self.replace(constraint.iter_terms(), constraint.lower());
    }
}

impl<ValueT, ConstraintT> From<&ConstraintT> for LinearConstraint<ValueT>
where
    ConstraintT: LinearConstraintTrait<Value = ValueT>,
{
    fn from(constraint: &ConstraintT) -> Self {
        return Self {
            terms: constraint.iter_terms().collect(),
            lower: constraint.lower(),
        };
    }
}

impl<ValueT> LinearConstraintTrait for LinearConstraint<ValueT>
where
    ValueT: UnsignedIntegerTrait,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.terms.iter().cloned()
    }

    fn lower(&self) -> Self::Value {
        self.lower
    }
}

#[derive(Clone, Debug)]
pub struct ConstraintView<ValueT, IteratorT> {
    terms: IteratorT,
    lower: ValueT,
}

impl<ValueT, IteratorT> ConstraintView<ValueT, IteratorT> {
    pub fn new(terms: IteratorT, lower: ValueT) -> Self {
        Self { terms, lower }
    }
}

impl<ValueT, IteratorT> LinearConstraintTrait for ConstraintView<ValueT, IteratorT>
where
    ValueT: UnsignedIntegerTrait,
    IteratorT: Iterator<Item = (Literal, ValueT)> + Clone,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.terms.clone()
    }

    fn lower(&self) -> Self::Value {
        self.lower
    }
}

#[derive(Default, Clone)]
pub struct RandomLinearConstraint<ValueT>
where
    ValueT: Copy,
{
    terms: Map<(Boolean, ValueT)>,
    lower: ValueT,
}

impl<ValueT> LinearConstraintTrait for RandomLinearConstraint<ValueT>
where
    ValueT: UnsignedIntegerTrait,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        return self
            .terms
            .iter()
            .map(|(&index, term)| (Literal::new(index, term.0), term.1.clone()));
    }

    fn lower(&self) -> Self::Value {
        return self.lower;
    }
}

impl<ValueT> RandomLinearConstraint<ValueT>
where
    ValueT: UnsignedIntegerTrait,
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

    pub fn replace_by_constraint(
        &mut self,
        constraint: impl LinearConstraintTrait<Value = ValueT>,
    ) {
        self.terms.clear();
        self.terms.extend(
            constraint
                .iter_terms()
                .filter(|(_, coefficient)| *coefficient != ValueT::zero())
                .map(|(literal, coefficient)| (literal.index(), (literal.value(), coefficient))),
        );
        self.lower = constraint.lower();
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

    pub fn add_assign(&mut self, rhs: &impl LinearConstraintTrait<Value = ValueT>) {
        self.lower = self.lower + rhs.lower();
        for (rhs_literal, rhs_coefficient) in rhs.iter_terms() {
            if let Some((lhs_literal, lhs_coefficient)) = self.terms.get_mut(rhs_literal.index()) {
                if *lhs_literal == rhs_literal.value() {
                    *lhs_coefficient += rhs_coefficient;
                } else {
                    if *lhs_coefficient > rhs_coefficient {
                        self.lower -= rhs_coefficient;
                        *lhs_coefficient -= rhs_coefficient;
                    } else if *lhs_coefficient < rhs_coefficient {
                        self.lower -= *lhs_coefficient;
                        *lhs_literal = rhs_literal.value();
                        *lhs_coefficient = rhs_coefficient - *lhs_coefficient;
                    } else {
                        self.lower -= rhs_coefficient;
                        self.terms.remove(rhs_literal.index());
                    }
                }
            } else {
                self.terms.insert(rhs_literal.index(), (rhs_literal.value(), rhs_coefficient));
            }
        }
    }
}

pub struct StrengthenConstraint<ValueT> {
    terms: Vec<(Literal, ValueT)>,
}

impl<ValueT> Default for StrengthenConstraint<ValueT> {
    fn default() -> Self {
        Self {
            terms: Vec::default(),
        }
    }
}

impl<ValueT> Clone for StrengthenConstraint<ValueT> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<ValueT> StrengthenConstraint<ValueT>
where
    ValueT: UnsignedIntegerTrait,
{
    pub fn exec<ExplainKeyT>(
        &mut self,
        constraint: &impl LinearConstraintTrait<Value = ValueT>,
        decision_stack: &DecisionStack<ExplainKeyT>,
    ) -> impl LinearConstraintTrait<Value = ValueT> + '_ {
        // 決定レベル 0 で割り当てられている変数を除いて制約を複製
        self.terms.clear();
        let mut lower = constraint.lower();
        for (literal, coefficient) in constraint.iter_terms() {
            if decision_stack.get_decision_level(literal.index()) == 0 {
                if decision_stack.is_true(literal) {
                    if lower <= coefficient {
                        // 右辺値が 0 になる場合には自明に満たされる制約条件なので両辺を 0 にして中断
                        self.terms.clear();
                        lower = ValueT::zero();
                        break;
                    } else {
                        lower = lower - coefficient;
                    }
                }
            } else {
                self.terms.push((literal, coefficient));
            }
        }
        // lower より大きい係数を lower に一致させる
        for (_, coefficient) in self.terms.iter_mut() {
            if *coefficient > lower {
                *coefficient = lower;
            }
        }
        // 係数が 0 である項を除く
        self.terms.retain(|&(_, coefficient)| coefficient > ValueT::zero());
        // 係数の最大公約数を使って丸め
        if self.terms.len() >= 1 {
            let mut gcd = ValueT::zero();
            for (_, coefficient) in self.terms.iter() {
                gcd = gcd.gcd(coefficient);
            }
            for (_, coefficient) in self.terms.iter_mut() {
                debug_assert!(*coefficient % gcd == ValueT::zero());
                *coefficient = *coefficient / gcd;
            }
            lower = lower.div_ceil(&gcd);
        }
        // 下限より小さい係数の合計を算出
        let mut sum_of_unsaturating_coefficients = ValueT::zero();
        for (_, coefficient) in self.terms.iter_mut() {
            if *coefficient < lower {
                sum_of_unsaturating_coefficients = sum_of_unsaturating_coefficients + *coefficient;
            }
        }
        // 下限より小さい係数の合計が下限未満であればそれらは 0 に切り下げることができる
        if sum_of_unsaturating_coefficients < lower {
            self.terms.retain(|&(_, coefficient)| coefficient == lower);
            for (_, coefficient) in self.terms.iter_mut() {
                *coefficient = ValueT::one();
            }
            lower = ValueT::one();
        }
        return ConstraintView::new(self.terms.iter().cloned(), lower);
    }
}
