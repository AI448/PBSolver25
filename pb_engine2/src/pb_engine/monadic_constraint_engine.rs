use num::{FromPrimitive, One};
use std::{collections::VecDeque, ops::Deref};

use super::{
    decision_stack::DecisionStack,
    etc::{Reason, State},
};
use crate::{
    Boolean, Literal,
    constraint::{ConstraintView, LinearConstraintTrait, UnsignedIntegerTrait},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OneSatEngineExplainKey {
    row_id: usize,
}

#[derive(Clone, Debug)]
pub struct OneSatEngine<CompositeExplainKeyT> {
    decision_stack: DecisionStack<CompositeExplainKeyT>,
    rows: Vec<Row>,
    state: State<OneSatEngineExplainKey>,
    constraint_queue: VecDeque<OneSatConstraint>,
}

impl<CompositeExplainKeyT> OneSatEngine<CompositeExplainKeyT> {
    pub fn new() -> Self {
        Self {
            decision_stack: DecisionStack::default(),
            rows: Vec::default(),
            state: State::Noconflict,
            constraint_queue: VecDeque::default(),
        }
    }
}

impl<CompositeExplainKeyT> Deref for OneSatEngine<CompositeExplainKeyT> {
    type Target = DecisionStack<CompositeExplainKeyT>;
    fn deref(&self) -> &Self::Target {
        return &self.decision_stack;
    }
}

impl<CompositeExplainKeyT> OneSatEngine<CompositeExplainKeyT>
where
    CompositeExplainKeyT: From<OneSatEngineExplainKey>,
{
    pub fn state(&self) -> State<OneSatEngineExplainKey> {
        return self.state;
    }

    pub fn explain<ValueT>(
        &self,
        explain_key: OneSatEngineExplainKey,
    ) -> impl LinearConstraintTrait<Value = ValueT>
    where
        ValueT: UnsignedIntegerTrait,
    {
        let constraint = &self.rows[explain_key.row_id].constraint;
        return ConstraintView::new(
            constraint.literals.iter().map(|&literal| (literal, ValueT::one())),
            ValueT::from(constraint.literals.len()).unwrap(),
        );
    }

    pub fn add_variable(&mut self) {
        self.decision_stack.add_variable(Boolean::FALSE);
    }

    pub fn assign(&mut self, literal: Literal, reason: Reason<CompositeExplainKeyT>) {
        assert!(self.state.is_noconflict());
        self.decision_stack.assign(literal, reason);
    }

    pub fn backjump(&mut self, backjump_level: usize) {
        self.decision_stack.backjump(backjump_level);
        if self.state.is_backjump_required() && backjump_level == 0 {
            self.state = State::Noconflict;
            while self.state.is_noconflict() && !self.constraint_queue.is_empty() {
                let constraint = self.constraint_queue.pop_front().unwrap();
                self.add_row(constraint);
            }
        }
    }

    pub fn add_constraint<ConstraintT>(&mut self, linear_constraint: &ConstraintT, _is_learnt: bool)
    where
        ConstraintT: LinearConstraintTrait,
    {
        assert!(
            linear_constraint
                .iter_terms()
                .all(|(_, coefficient)| coefficient == ConstraintT::Value::one())
        );
        assert!(
            linear_constraint.lower()
                == ConstraintT::Value::from_usize(linear_constraint.iter_terms().count()).unwrap()
        );

        let constraint = OneSatConstraint {
            literals: Vec::from_iter(linear_constraint.iter_terms().map(|(literal, _)| literal)),
        };

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

    fn add_row(&mut self, constraint: OneSatConstraint) {
        debug_assert!(self.state.is_noconflict());
        debug_assert!(self.decision_stack.decision_level() == 0);

        let row_id = self.rows.len();
        self.rows.push(Row { constraint });
        let row = &self.rows[row_id];

        let explain_key = OneSatEngineExplainKey { row_id };
        for &literal in row.constraint.literals.iter() {
            if !self.decision_stack.is_assigned(literal.index()) {
                // 未割り当てであれば真を割り当て
                self.decision_stack.assign(
                    literal,
                    Reason::Propagation {
                        explain_key: explain_key.into(),
                    },
                );
            } else if self.decision_stack.is_false(literal) {
                // すでに False が割あたっていれば state を Conflict にして中断
                self.state.merge(State::Conflict { explain_key });
                break;
            }
        }
    }
}

#[derive(Clone, Debug)]
struct OneSatConstraint {
    literals: Vec<Literal>,
}

#[derive(Clone, Debug)]
struct Row {
    constraint: OneSatConstraint,
}
