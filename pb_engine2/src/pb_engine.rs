mod count_constraint_engine;
mod decision_stack;
mod engine_trait;
mod etc;
mod linear_constraint_engine;
mod monadic_constraint_engine;

use std::{fmt::Debug, ops::Deref};

use either::Either;
use num::{FromPrimitive, Integer, One, ToPrimitive, Unsigned};
use utility::Map;

use crate::{Literal, activities::Activities};
pub use count_constraint_engine::{
    CountConstraint, CountConstraintEngine, CountConstraintExplainKey, CountConstraintTrait,
    CountConstraintView,
};
pub use decision_stack::DecisionStack;
pub use engine_trait::{EngineAddConstraintTrait, EngineTrait};
pub use etc::{Reason, State};
pub use linear_constraint_engine::{
    LinearConstraint, LinearConstraintEngine, LinearConstraintExplainKey, LinearConstraintTrait,
    LinearConstraintView, RandomAccessibleLinearConstraint,
};
pub use monadic_constraint_engine::{
    MonadicConstraint, MonadicConstraintEngine, MonadicConstraintExplainKey,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PBExplainKey {
    MonadicClause(MonadicConstraintExplainKey),
    CountConstraint(CountConstraintExplainKey),
    LinearConstraint(LinearConstraintExplainKey),
}

impl From<MonadicConstraintExplainKey> for PBExplainKey {
    fn from(explain_key: MonadicConstraintExplainKey) -> Self {
        Self::MonadicClause(explain_key)
    }
}

impl From<CountConstraintExplainKey> for PBExplainKey {
    fn from(explain_key: CountConstraintExplainKey) -> Self {
        Self::CountConstraint(explain_key)
    }
}

impl From<LinearConstraintExplainKey> for PBExplainKey {
    fn from(explain_key: LinearConstraintExplainKey) -> Self {
        Self::LinearConstraint(explain_key)
    }
}

impl
    From<
        Either<
            LinearConstraintExplainKey,
            Either<CountConstraintExplainKey, MonadicConstraintExplainKey>,
        >,
    > for PBExplainKey
{
    fn from(
        value: Either<
            LinearConstraintExplainKey,
            Either<CountConstraintExplainKey, MonadicConstraintExplainKey>,
        >,
    ) -> Self {
        match value {
            Either::Left(explain_key) => Self::LinearConstraint(explain_key),
            Either::Right(explain_key) => match explain_key {
                Either::Left(explain_key) => Self::CountConstraint(explain_key),
                Either::Right(explain_key) => Self::MonadicClause(explain_key),
            },
        }
    }
}

impl From<PBExplainKey>
    for Either<
        LinearConstraintExplainKey,
        Either<CountConstraintExplainKey, MonadicConstraintExplainKey>,
    >
{
    fn from(explain_key: PBExplainKey) -> Self {
        match explain_key {
            PBExplainKey::LinearConstraint(explain_key) => Either::Left(explain_key),
            PBExplainKey::CountConstraint(explain_key) => Either::Right(Either::Left(explain_key)),
            PBExplainKey::MonadicClause(explain_key) => Either::Right(Either::Right(explain_key)),
        }
    }
}

pub struct PBConstraint<LinearConstraintT, CountConstraintT>(
    Either<LinearConstraintT, Either<CountConstraintT, MonadicConstraint>>,
);

impl<LinearConstraintT, CountConstraintT> LinearConstraintTrait
    for PBConstraint<LinearConstraintT, CountConstraintT>
where
    LinearConstraintT: LinearConstraintTrait,
    CountConstraintT: CountConstraintTrait,
{
    type Value = LinearConstraintT::Value;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        return match &self.0 {
            Either::Left(linear_constraint) => Either::Left(linear_constraint.iter_terms()),
            Either::Right(constraint) => match constraint {
                Either::Left(count_constraint) => Either::Right(Either::Left(
                    count_constraint.iter_terms().map(|literal| (literal, Self::Value::one())),
                )),
                Either::Right(monadic_constraint) => Either::Right(Either::Right(
                    [(monadic_constraint.literal, Self::Value::one())].into_iter(),
                )),
            },
        };
    }

    fn lower(&self) -> Self::Value {
        return match &self.0 {
            Either::Left(linear_constraint) => linear_constraint.lower(),
            Either::Right(constraint) => match constraint {
                Either::Left(count_constraint) => {
                    Self::Value::from_usize(count_constraint.lower()).unwrap()
                }
                Either::Right(_monadic_constraint) => Self::Value::one(),
            },
        };
    }
}

pub struct PBEngine<ValueT> {
    linear_cosntraint_to_pb_constraint: LinearConstraintToPBConstraint<ValueT>,
    inner_engine: LinearConstraintEngine<
        ValueT,
        CountConstraintEngine<MonadicConstraintEngine<PBExplainKey>>,
    >,
    // TODO: Activities は PBEngine の外に出す
    activities: Activities,
    variable_map: Map<f64>,
}

impl<ValueT> PBEngine<ValueT> {
    pub fn new() -> Self {
        Self {
            linear_cosntraint_to_pb_constraint: LinearConstraintToPBConstraint::default(),
            inner_engine: LinearConstraintEngine::new(CountConstraintEngine::new(
                MonadicConstraintEngine::new(),
            )),
            activities: Activities::new(1e1),
            variable_map: Map::default(),
        }
    }
}

impl<ValueT> Deref for PBEngine<ValueT>
where
    ValueT: Integer + Unsigned + Copy + Debug,
{
    type Target = DecisionStack<PBExplainKey>;
    fn deref(&self) -> &Self::Target {
        self.inner_engine.deref()
    }
}

impl<ValueT> PBEngine<ValueT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive + ToPrimitive + Debug,
{
    pub fn assignment_probability(&self, literal: Literal) -> f64 {
        return self.activities.assignment_probability(literal);
    }

    pub fn activity(&self, index: usize) -> f64 {
        return self.activities.activity(index);
    }

    pub fn update_assignment_probabilities(&mut self) {
        self.activities.update_assignment_probabilities(
            (0..self.inner_engine.number_of_assignments())
                .map(|order| self.inner_engine.get_assignment(order)),
        );
    }

    pub fn update_conflict_probabilities(
        &mut self,
        conflict_assignments: impl Iterator<Item = Literal>,
    ) {
        self.variable_map.clear();
        for literal in conflict_assignments {
            self.variable_map.insert(literal.index(), 1.0);
        }
        for literal in (0..self.inner_engine.number_of_assignments())
            .map(|order| self.inner_engine.get_assignment(order))
        {
            if !self.variable_map.contains_key(literal.index()) {
                self.variable_map.insert(literal.index(), 0.0);
            }
        }

        for (&index, &value) in self.variable_map.iter() {
            self.activities.update_activity(index, value);
        }
    }

    pub fn state(&self) -> State<PBExplainKey> {
        match self.inner_engine.state() {
            State::BackjumpRequired { backjump_level } => {
                State::BackjumpRequired { backjump_level }
            }
            State::Conflict { explain_key } => State::Conflict {
                explain_key: explain_key.into(),
            },
            State::Noconflict => State::Noconflict,
        }
    }

    pub fn explain(&self, explain_key: PBExplainKey) -> impl LinearConstraintTrait<Value = ValueT> {
        PBConstraint(self.inner_engine.explain(explain_key.into()))
    }

    pub fn add_variable(&mut self) {
        self.inner_engine.add_variable();
        self.activities.add_variable(0.0);
    }

    // pub fn decide(&mut self, decision: Literal) {
    //     assert!(self.inner_engine.state().is_noconflict());
    //     self.inner_engine.assign(decision, Reason::Decision);
    // }

    pub fn decide(&mut self) {
        assert!(self.inner_engine.state().is_noconflict());
        let decision_variable = {
            let mut decision_variable = None;
            loop {
                let variable = self.activities.pop_unassigned_variable().unwrap();
                if !self.inner_engine.is_assigned(variable) {
                    decision_variable.replace(variable);
                    break;
                }
            }
            decision_variable.unwrap()
        };
        let decision_value = self.inner_engine.get_value(decision_variable);
        self.inner_engine.assign(
            Literal::new(decision_variable, decision_value),
            Reason::Decision,
        );
    }

    pub fn backjump(&mut self, backjump_level: usize) {
        assert!(backjump_level < self.inner_engine.decision_level());

        for order in self.inner_engine.order_range(backjump_level).end
            ..self.inner_engine.number_of_assignments()
        {
            self.activities
                .push_unassigned_variable(self.inner_engine.get_assignment(order).index());
        }

        self.inner_engine.backjump(backjump_level);
    }

    pub fn add_constraint(
        &mut self,
        constraint: &impl LinearConstraintTrait<Value = ValueT>,
        is_learnt: bool,
    ) {
        let Some(pb_constraint) =
            self.linear_cosntraint_to_pb_constraint.exec(constraint, &self.inner_engine)
        else {
            return;
        };
        if is_learnt {
            if let Either::Right(x) = &pb_constraint {
                eprintln!("{}", matches!(x, Either::Left(..)));
            } else {
                eprintln!("ADD_LINEAR_CONSTRAINT");
            }
        }
        self.inner_engine.add_constraint(pb_constraint, is_learnt);
    }
}

#[derive(Clone, Debug)]
struct LinearConstraintToPBConstraint<ValueT> {
    terms: Vec<(Literal, ValueT)>,
}

impl<ValueT> Default for LinearConstraintToPBConstraint<ValueT> {
    fn default() -> Self {
        Self {
            terms: Vec::default(),
        }
    }
}

impl<ValueT> LinearConstraintToPBConstraint<ValueT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive + ToPrimitive,
{
    pub fn exec<ExplainKeyT: Copy>(
        &mut self,
        constraint: &impl LinearConstraintTrait<Value = ValueT>,
        decision_stack: &DecisionStack<ExplainKeyT>,
    ) -> Option<
        Either<
            impl LinearConstraintTrait<Value = ValueT> + '_,
            Either<impl CountConstraintTrait + '_, MonadicConstraint>,
        >,
    > {
        self.terms.clear();

        // 決定レベル 0 で割り当てられている項を除いて terms にコピー
        let mut lower = constraint.lower();
        for (literal, coefficient) in constraint.iter_terms() {
            if decision_stack.get_decision_level(literal.index()) == 0 {
                if decision_stack.is_true(literal) {
                    if lower <= coefficient {
                        return None;
                    }
                    lower = lower - coefficient;
                }
            } else if coefficient != ValueT::zero() {
                self.terms.push((literal, coefficient));
            }
        }

        if lower == ValueT::zero() {
            return None;
        } else {
            // 下限より大きい係数を下限に一致させる
            for (_, coefficient) in self.terms.iter_mut() {
                if *coefficient > lower {
                    *coefficient = lower;
                }
            }
        }

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
        };

        // 下限より小さい係数の合計を算出
        let mut sum_of_unsaturating_coefficients = ValueT::zero();
        for (_, coefficient) in self.terms.iter_mut() {
            if *coefficient < lower {
                sum_of_unsaturating_coefficients = sum_of_unsaturating_coefficients + *coefficient;
            }
        }

        let f = |&(literal, _)| literal;
        if sum_of_unsaturating_coefficients < lower {
            // 下限より小さい係数の合計が下限未満であれば，それらの項は無視できるため，
            // 係数が下限に一致する項のみを抽出して MonadicConstraint または CountConstraint を返す
            self.terms.retain(|&(_, coefficient)| coefficient == lower);
            if self.terms.len() == 1 {
                return Some(Either::Right(Either::Right(MonadicConstraint {
                    literal: self.terms[0].0,
                })));
            } else {
                return Some(Either::Right(Either::Left(CountConstraintView::new(
                    self.terms.iter().map(f),
                    1,
                ))));
            }
        } else if self.terms.iter().all(|&(_, coefficient)| coefficient == ValueT::one()) {
            return Some(Either::Right(Either::Left(CountConstraintView::new(
                self.terms.iter().map(f),
                lower.to_usize().unwrap(),
            ))));
        } else {
            // 線形制約を返す
            return Some(Either::Left(LinearConstraintView::new(
                self.terms.iter().cloned(),
                lower,
            )));
        }
    }
}
