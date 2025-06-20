use std::ops::Deref;

use super::{
    decision_stack::DecisionStack,
    etc::{Reason, State},
};
use crate::Literal;

pub trait EngineTrait: Deref<Target = DecisionStack<Self::CompositeExplainKey>> {
    type CompositeExplainKey: Copy;
    type ExplainKey: Copy;
    type ExplanationConstraint<'a>
    where
        Self: 'a;

    fn state(&self) -> State<Self::ExplainKey>;

    fn explain(&self, explain_key: Self::ExplainKey) -> Self::ExplanationConstraint<'_>;

    fn add_variable(&mut self);

    fn assign(&mut self, literal: Literal, reason: Reason<Self::CompositeExplainKey>);

    fn backjump(&mut self, backjump_level: usize);
}

pub trait EngineAddConstraintTrait<ConstraintT>: EngineTrait {
    fn add_constraint(&mut self, constraint: ConstraintT, is_learnt: bool);
}
