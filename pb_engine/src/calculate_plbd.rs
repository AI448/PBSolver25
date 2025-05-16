use utility::Set;

use crate::{Literal, decision_stack::DecisionStack};

#[derive(Default)]
pub struct CalculatePLBD {
    decision_level_set: Set,
}

impl Clone for CalculatePLBD {
    fn clone(&self) -> Self {
        Self {
            decision_level_set: Set::default(),
        }
    }
}

impl CalculatePLBD {
    pub fn calculate(
        &mut self,
        assignments: impl Iterator<Item = Literal>,
        decision_stack: &DecisionStack<impl Copy>,
    ) -> usize {
        self.decision_level_set.clear();
        for assignment in assignments {
            debug_assert!(decision_stack.is_true(assignment));
            self.decision_level_set
                .insert(decision_stack.get_decision_level(assignment.index()));
        }
        return self.decision_level_set.len();
    }
}
