use std::{collections::VecDeque, ops::Deref};

use super::{
    decision_stack::DecisionStack,
    engine_trait::{EngineAddConstraintTrait, EngineTrait},
    etc::{Reason, State},
};
use crate::{Boolean, Literal};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MonadicConstraint {
    pub literal: Literal,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MonadicConstraintExplainKey {
    constraint: MonadicConstraint,
}

#[derive(Clone, Debug)]
pub struct MonadicConstraintEngine<CompositeExplainKeyT> {
    state: State<MonadicConstraintExplainKey>,
    decision_stack: DecisionStack<CompositeExplainKeyT>,
    constraint_queue: VecDeque<MonadicConstraint>,
}

impl<CompositeExplainKeyT> MonadicConstraintEngine<CompositeExplainKeyT> {
    pub fn new() -> Self {
        Self {
            state: State::Noconflict,
            decision_stack: DecisionStack::default(),
            constraint_queue: VecDeque::default(),
        }
    }
}

impl<CompositeExplainKeyT> Deref for MonadicConstraintEngine<CompositeExplainKeyT> {
    type Target = DecisionStack<CompositeExplainKeyT>;
    fn deref(&self) -> &Self::Target {
        return &self.decision_stack;
    }
}

impl<CompositeExplainKeyT> EngineTrait for MonadicConstraintEngine<CompositeExplainKeyT>
where
    CompositeExplainKeyT: Copy + From<MonadicConstraintExplainKey>,
{
    type CompositeExplainKey = CompositeExplainKeyT;

    type ExplainKey = MonadicConstraintExplainKey;

    type ExplanationConstraint<'a>
        = MonadicConstraint
    where
        Self: 'a;

    fn state(&self) -> State<Self::ExplainKey> {
        return self.state;
    }

    fn explain(&self, explain_key: MonadicConstraintExplainKey) -> MonadicConstraint {
        return explain_key.constraint;
    }

    fn add_variable(&mut self) {
        self.decision_stack.add_variable(Boolean::FALSE);
    }

    fn assign(&mut self, literal: Literal, reason: Reason<Self::CompositeExplainKey>) {
        assert!(self.state.is_noconflict());
        self.decision_stack.assign(literal, reason);
    }

    fn backjump(&mut self, backjump_level: usize) {
        self.decision_stack.backjump(backjump_level);
        if self.state.is_backjump_required() && backjump_level == 0 {
            self.state = State::Noconflict;
            while self.state.is_noconflict() && !self.constraint_queue.is_empty() {
                let constraint = self.constraint_queue.pop_front().unwrap();
                self.add_row(constraint);
            }
        }
    }
}

impl<CompositeExplainKeyT: Copy + From<MonadicConstraintExplainKey>>
    EngineAddConstraintTrait<MonadicConstraint> for MonadicConstraintEngine<CompositeExplainKeyT>
{
    fn add_constraint(&mut self, constraint: MonadicConstraint, _is_learnt: bool) {
        if self.decision_stack.decision_level() == 0 {
            if self.state.is_noconflict() {
                self.add_row(constraint);
            } else {
                self.constraint_queue.push_back(constraint);
            }
        } else {
            self.constraint_queue.push_back(constraint);
            self.state = State::BackjumpRequired { backjump_level: 0 };
        }
    }
}

impl<CompositeExplainKeyT> MonadicConstraintEngine<CompositeExplainKeyT>
where
    CompositeExplainKeyT: Copy + From<MonadicConstraintExplainKey>,
{
    fn add_row(&mut self, constraint: MonadicConstraint) {
        debug_assert!(self.state.is_noconflict());
        debug_assert!(self.decision_stack.decision_level() == 0);

        let explain_key = MonadicConstraintExplainKey { constraint };
        if !self.decision_stack.is_assigned(constraint.literal.index()) {
            // 未割り当てであれば
            self.decision_stack.assign(
                constraint.literal,
                Reason::Propagation {
                    explain_key: explain_key.into(),
                },
            );
        } else if self.decision_stack.is_false(constraint.literal) {
            // すでに False が割あたっていれば state を Conflict に
            self.state.merge(State::Conflict { explain_key });
        }
    }
}
