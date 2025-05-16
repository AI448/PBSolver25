use crate::{MonadicClause, decision_stack::DecisionStack};

use super::{Propagation, TheoryAddConstraintTrait, TheoryTrait};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MonadicClauseExplainKey {
    monadic_clause: MonadicClause,
}

#[derive(Clone)]
pub struct MonadicClauseTheory {
    monadic_clauses: Vec<MonadicClause>,
    number_of_evaluated_assignments: usize,
}

impl MonadicClauseTheory {
    pub fn new() -> Self {
        Self {
            monadic_clauses: Vec::default(),
            number_of_evaluated_assignments: 0,
        }
    }

    pub fn number_of_monadic_clauses(&self) -> usize {
        return self.monadic_clauses.len();
    }
}

impl TheoryTrait for MonadicClauseTheory {
    type ExplainKey = MonadicClauseExplainKey;
    type ExplanationConstraint<'a> = MonadicClause;

    fn add_variable(&mut self) {}

    fn assign<ExplainKeyT: Copy>(
        &mut self,
        decision_stack: &DecisionStack<ExplainKeyT>,
        _callback: impl FnMut(Propagation<Self::ExplainKey>),
    ) {
        assert!(
            decision_stack.number_of_assignments() == self.number_of_evaluated_assignments
                || decision_stack.number_of_assignments()
                    == self.number_of_evaluated_assignments + 1
        );
        self.number_of_evaluated_assignments = decision_stack.number_of_assignments();
    }

    fn backjump<ExplainKeyT: Copy>(
        &mut self,
        backjump_level: usize,
        decision_stack: &DecisionStack<ExplainKeyT>,
    ) {
        let backjump_order = decision_stack.order_range(backjump_level).end;
        assert!(backjump_order <= self.number_of_evaluated_assignments);
        self.number_of_evaluated_assignments = backjump_order;
    }

    fn explain(&self, explain_key: Self::ExplainKey) -> Self::ExplanationConstraint<'_> {
        explain_key.monadic_clause
    }
}

impl TheoryAddConstraintTrait<MonadicClause> for MonadicClauseTheory {
    fn add_constraint<ExplainKeyT: Copy>(
        &mut self,
        constraint: MonadicClause,
        assignment_state: &DecisionStack<ExplainKeyT>,
        mut callback: impl FnMut(Propagation<Self::ExplainKey>),
    ) -> Result<(), usize> {
        assert!(assignment_state.number_of_assignments() == self.number_of_evaluated_assignments);
        if assignment_state.decision_level() != 0 {
            return Err(0);
        } else {
            self.monadic_clauses.push(constraint);
            assert!(!assignment_state.is_false(constraint.literal)); // TODO 後で考える
            callback(Propagation {
                literal: constraint.literal,
                explain_key: MonadicClauseExplainKey {
                    monadic_clause: constraint,
                },
                plbd: 0,
            });
            return Ok(());
        }
    }
}
