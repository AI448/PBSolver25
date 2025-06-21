mod count_constraint_engine;
mod decision_stack;
// mod engine_trait;
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
// pub use engine_trait::{EngineAddConstraintTrait, EngineTrait};
pub use etc::{Reason, State};
pub use linear_constraint_engine::{
    CompositeLinearConstraint, LinearConstraint, LinearConstraintEngine,
    LinearConstraintEngineExplainKey, LinearConstraintExplainKey, LinearConstraintTrait,
    LinearConstraintView, RandomAccessibleLinearConstraint,
};
pub use monadic_constraint_engine::{
    MonadicConstraint, MonadicConstraintEngine, MonadicConstraintExplainKey,
};

// #[derive(Clone, Copy, PartialEq, Eq, Debug)]
// pub enum PBExplainKey {
//     MonadicClause(MonadicConstraintExplainKey),
//     CountConstraint(CountConstraintExplainKey),
//     LinearConstraint(LinearConstraintExplainKey),
// }

// impl From<MonadicConstraintExplainKey> for PBExplainKey {
//     fn from(explain_key: MonadicConstraintExplainKey) -> Self {
//         Self::MonadicClause(explain_key)
//     }
// }

// impl From<CountConstraintExplainKey> for PBExplainKey {
//     fn from(explain_key: CountConstraintExplainKey) -> Self {
//         Self::CountConstraint(explain_key)
//     }
// }

// impl From<LinearConstraintExplainKey> for PBExplainKey {
//     fn from(explain_key: LinearConstraintExplainKey) -> Self {
//         Self::LinearConstraint(explain_key)
//     }
// }

// impl
//     From<
//         Either<
//             LinearConstraintExplainKey,
//             Either<CountConstraintExplainKey, MonadicConstraintExplainKey>,
//         >,
//     > for PBExplainKey
// {
//     fn from(
//         value: Either<
//             LinearConstraintExplainKey,
//             Either<CountConstraintExplainKey, MonadicConstraintExplainKey>,
//         >,
//     ) -> Self {
//         match value {
//             Either::Left(explain_key) => Self::LinearConstraint(explain_key),
//             Either::Right(explain_key) => match explain_key {
//                 Either::Left(explain_key) => Self::CountConstraint(explain_key),
//                 Either::Right(explain_key) => Self::MonadicClause(explain_key),
//             },
//         }
//     }
// }

// impl From<PBExplainKey>
//     for Either<
//         LinearConstraintExplainKey,
//         Either<CountConstraintExplainKey, MonadicConstraintExplainKey>,
//     >
// {
//     fn from(explain_key: PBExplainKey) -> Self {
//         match explain_key {
//             PBExplainKey::LinearConstraint(explain_key) => Either::Left(explain_key),
//             PBExplainKey::CountConstraint(explain_key) => Either::Right(Either::Left(explain_key)),
//             PBExplainKey::MonadicClause(explain_key) => Either::Right(Either::Right(explain_key)),
//         }
//     }
// }

pub type PBExplainKey = LinearConstraintEngineExplainKey;

// pub struct PBConstraint<LinearConstraintT, CountConstraintT>(
//     Either<LinearConstraintT, Either<CountConstraintT, MonadicConstraint>>,
// );

// impl<LinearConstraintT, CountConstraintT> LinearConstraintTrait
//     for PBConstraint<LinearConstraintT, CountConstraintT>
// where
//     LinearConstraintT: LinearConstraintTrait,
//     CountConstraintT: CountConstraintTrait,
// {
//     type Value = LinearConstraintT::Value;
//     fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
//         return match &self.0 {
//             Either::Left(linear_constraint) => Either::Left(linear_constraint.iter_terms()),
//             Either::Right(constraint) => match constraint {
//                 Either::Left(count_constraint) => Either::Right(Either::Left(
//                     count_constraint.iter_terms().map(|literal| (literal, Self::Value::one())),
//                 )),
//                 Either::Right(monadic_constraint) => Either::Right(Either::Right(
//                     [(monadic_constraint.literal, Self::Value::one())].into_iter(),
//                 )),
//             },
//         };
//     }

//     fn lower(&self) -> Self::Value {
//         return match &self.0 {
//             Either::Left(linear_constraint) => linear_constraint.lower(),
//             Either::Right(constraint) => match constraint {
//                 Either::Left(count_constraint) => {
//                     Self::Value::from_usize(count_constraint.lower()).unwrap()
//                 }
//                 Either::Right(_monadic_constraint) => Self::Value::one(),
//             },
//         };
//     }
// }

pub struct PBEngine {
    inner_engine: LinearConstraintEngine<PBExplainKey>,
    // TODO: Activities は PBEngine の外に出す
    activities: Activities,
    variable_map: Map<f64>,
}

impl PBEngine {
    pub fn new() -> Self {
        Self {
            inner_engine: LinearConstraintEngine::new(),
            activities: Activities::new(1e1),
            variable_map: Map::default(),
        }
    }
}

impl Deref for PBEngine {
    type Target = DecisionStack<PBExplainKey>;
    fn deref(&self) -> &Self::Target {
        self.inner_engine.deref()
    }
}

impl PBEngine {
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

    pub fn explain(&self, explain_key: PBExplainKey) -> impl LinearConstraintTrait<Value = u64> {
        self.inner_engine.explain(explain_key.into())
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
        constraint: &impl LinearConstraintTrait<Value = u64>,
        is_learnt: bool,
    ) {
        self.inner_engine.add_constraint(constraint, is_learnt);
    }
}
