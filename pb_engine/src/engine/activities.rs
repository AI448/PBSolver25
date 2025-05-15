use crate::{Literal, collections::LiteralArray};

pub struct Activities {
    time_constant: f64,
    assignment_probabilities: LiteralArray<f64>,
    conflict_probabilities: LiteralArray<f64>,
    activities: Vec<f64>,
    activity_increase_value: f64,
}

impl Activities {
    pub fn new(time_constant: f64) -> Self {
        assert!(time_constant > 0.0);
        Self {
            time_constant,
            assignment_probabilities: LiteralArray::default(),
            conflict_probabilities: LiteralArray::default(),
            activities: Vec::default(),
            activity_increase_value: 1.0,
        }
    }

    pub fn add_variable(&mut self) {
        self.assignment_probabilities.push([0.0, 0.0]);
        self.conflict_probabilities.push([0.0, 0.0]);
        self.activities.push(0.0);
    }

    pub fn update_assignment_probabilities(&mut self, assignments: impl Iterator<Item = Literal>) {
        let r = 1.0 - 1.0 / self.time_constant;
        for [p, q] in self.assignment_probabilities.iter_mut() {
            *p *= r;
            *q *= r;
        }
        for assignment in assignments {
            self.assignment_probabilities[assignment] += 1.0 - r;
            debug_assert!(self.assignment_probabilities[assignment].is_finite(), "{} {}", self.assignment_probabilities[assignment], r);
        }
    }

    pub fn update_conflict_probabilities(
        &mut self,
        conflict_assignments: impl Iterator<Item = Literal>,
    ) {
        self.activity_increase_value /= 1.0 - 1.0 / self.time_constant;
        if self.activity_increase_value > 1e4 {
            for activity in self.activities.iter_mut() {
                *activity /= self.activity_increase_value;
            }
            self.activity_increase_value = 1.0;
        }

        let r = 1.0 - 1.0 / self.time_constant;
        for [p, q] in self.conflict_probabilities.iter_mut() {
            *p *= r;
            *q *= r;
        }

        for assignment in conflict_assignments {
            self.conflict_probabilities[assignment] += 1.0 - r;
            self.activities[assignment.index()] += self.activity_increase_value;
        }
    }

    pub fn assignment_probability(&self, literal: Literal) -> f64 {
        return self.assignment_probabilities[literal];
    }

    pub fn conflict_probability(&self, literal: Literal) -> f64 {
        return self.conflict_probabilities[literal];
    }

    pub fn activity(&self, index: usize) -> f64 {
        return self.activities[index] / self.activity_increase_value;
    }
}
