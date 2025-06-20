use utility::Set;

use crate::{Literal, pb_engine::DecisionStack};

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
    pub fn calculate<ExplainKeyT: Copy>(
        &mut self,
        assignments: impl Iterator<Item = Literal>,
        decision_stack: &DecisionStack<ExplainKeyT>,
    ) -> usize {
        self.decision_level_set.clear();
        for assignment in assignments {
            debug_assert!(decision_stack.is_true(assignment));
            let decision_level = decision_stack.get_decision_level(assignment.index());
            if decision_level != 0 {
                self.decision_level_set.insert(decision_level);
            }
        }
        return self.decision_level_set.len();
    }
}
