// TODO: Theory に見せる Trait と Engine のための実装とを分けたほうがいい？

use std::ops::Range;

use crate::{
    engine::Reason,
    types::{Boolean, Literal},
};

#[derive(Clone)]
pub struct DecisionStack<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    states: Vec<State>,
    assignment_stack: Vec<Assignment<ExplainKeyT>>,
    decision_stack: Vec<Decision>,
}

impl<ExplainKeyT> Default for DecisionStack<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    fn default() -> Self {
        Self {
            states: Vec::default(),
            assignment_stack: Vec::default(),
            decision_stack: Vec::default(),
        }
    }
}

impl<ExplainKeyT> DecisionStack<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    const NULL_ASSIGNMENT_ORDER: usize = usize::MAX;

    pub fn number_of_variables(&self) -> usize {
        return self.states.len();
    }

    pub fn number_of_assignments(&self) -> usize {
        return self.assignment_stack.len();
    }

    pub fn decision_level(&self) -> usize {
        return self.decision_stack.len();
    }

    pub fn is_assigned(&self, index: usize) -> bool {
        return self.states[index].order != Self::NULL_ASSIGNMENT_ORDER;
    }

    pub fn get_value(&self, index: usize) -> Boolean {
        return self.states[index].value;
    }

    pub fn is_true(&self, literal: Literal) -> bool {
        return self.states[literal.index()].order != Self::NULL_ASSIGNMENT_ORDER
            && self.states[literal.index()].value == literal.value();
    }

    pub fn is_false(&self, literal: Literal) -> bool {
        return self.states[literal.index()].order != Self::NULL_ASSIGNMENT_ORDER
            && self.states[literal.index()].value == !literal.value();
    }

    pub fn is_assigned_at(&self, index: usize, order: usize) -> bool {
        if order == Self::NULL_ASSIGNMENT_ORDER {
            return self.is_assigned(index);
        } else {
            return self.states[index].order <= order;
        }
    }

    pub fn is_true_at(&self, literal: Literal, order: usize) -> bool {
        if order == Self::NULL_ASSIGNMENT_ORDER {
            return self.is_true(literal);
        } else {
            return self.states[literal.index()].order <= order
            && self.states[literal.index()].value == literal.value();
        }
    }

    pub fn is_false_at(&self, literal: Literal, order: usize) -> bool {
        if order == Self::NULL_ASSIGNMENT_ORDER {
            return self.is_false(literal);
        } else {
            return self.states[literal.index()].order <= order
                && self.states[literal.index()].value == !literal.value();
        }
    }

    pub fn get_assignment_order(&self, index: usize) -> usize {
        return self.states[index].order;
    }

    pub fn get_decision_level(&self, index: usize) -> usize {
        let order = self.states[index].order;
        if order == Self::NULL_ASSIGNMENT_ORDER {
            return Self::NULL_ASSIGNMENT_ORDER;
        } else {
            return self.assignment_stack[order].decision_level;
        }
    }

    pub fn order_range(&self, decision_level: usize) -> Range<usize> {
        let start = if decision_level == 0 {
            0
        } else {
            self.decision_stack[decision_level - 1].assignment_order
        };
        let end = if decision_level < self.decision_stack.len() {
            self.decision_stack[decision_level].assignment_order
        } else {
            self.assignment_stack.len()
        };
        return std::ops::Range { start, end };
    }

    pub fn get_assignment(&self, order: usize) -> Literal {
        let index = self.assignment_stack[order].index;
        let value = self.states[index].value;
        debug_assert!(self.states[index].order == order);
        return Literal::new(index, value);
    }

    pub fn add_variable(&mut self, initial_value: Boolean) {
        self.states.push(State {
            value: initial_value,
            order: Self::NULL_ASSIGNMENT_ORDER,
        });
    }

    pub fn assign(&mut self, literal: Literal, reason: Reason<ExplainKeyT>) {
        debug_assert!(self.states[literal.index()].order == Self::NULL_ASSIGNMENT_ORDER);

        let assignment_order = self.assignment_stack.len();
        if reason.is_decision() {
            self.decision_stack.push(Decision { assignment_order });
        }
        let decision_level = self.decision_stack.len();
        self.assignment_stack.push(Assignment {
            index: literal.index(),
            decision_level,
            reason,
        });
        self.states[literal.index()].order = assignment_order;
        self.states[literal.index()].value = literal.value();
    }

    pub fn backjump(&mut self, backjump_level: usize) {
        while self.decision_stack.len() > backjump_level {
            let assignment = self.assignment_stack.pop().unwrap();
            debug_assert!(assignment.decision_level == self.decision_stack.len());
            if assignment.reason.is_decision() {
                let decision = self.decision_stack.pop().unwrap();
                debug_assert!(decision.assignment_order == self.assignment_stack.len());
            }
            // let unassigned_literal = Literal::new(
            //     assignment.index,
            //     self.states[assignment.index].value
            // );
            self.states[assignment.index].order = Self::NULL_ASSIGNMENT_ORDER;
        }
    }

    pub fn get_reason(&self, index: usize) -> Option<Reason<ExplainKeyT>> {
        let order = self.states[index].order;
        if order == Self::NULL_ASSIGNMENT_ORDER {
            return None;
        } else {
            let assignment = &self.assignment_stack[order];
            return Some(assignment.reason);
        }
    }
}

#[derive(Clone)]
struct Decision {
    assignment_order: usize,
}

#[derive(Clone)]
struct Assignment<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    index: usize,
    decision_level: usize,
    reason: Reason<ExplainKeyT>,
}

#[derive(Clone, Copy)]
struct State {
    value: Boolean,
    order: usize,
}
