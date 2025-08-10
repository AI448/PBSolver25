use utility::HeapedMap;

use crate::{Literal, collections::LiteralArray};

pub struct Activities {
    time_constant: f64,
    assignment_probabilities: LiteralArray<f64>,
    activities: Vec<f64>,
    // activity_increase_value: f64,
    unassigned_variables: HeapedMap<f64, CompareUnassignedVariables>,
}

impl Activities {
    pub fn new(time_constant: f64) -> Self {
        assert!(time_constant > 0.0);
        Self {
            time_constant,
            assignment_probabilities: LiteralArray::default(),
            activities: Vec::default(),
            // activity_increase_value: 1.0,
            unassigned_variables: HeapedMap::default(),
        }
    }

    pub fn add_variable(&mut self, initial_activity: f64) {
        let index = self.assignment_probabilities.len();
        self.assignment_probabilities.push([0.0, 0.0]);
        self.activities.push(initial_activity);
        self.unassigned_variables.insert(index, initial_activity);
    }

    pub fn update_assignment_probabilities(&mut self, assignments: impl Iterator<Item = Literal>) {
        let r = 1.0 - 1.0 / self.time_constant;
        for [p, q] in self.assignment_probabilities.iter_mut() {
            *p *= r;
            *q *= r;
        }
        for assignment in assignments {
            self.assignment_probabilities[assignment] += 1.0 - r;
            debug_assert!(
                self.assignment_probabilities[assignment].is_finite(),
                "{} {}",
                self.assignment_probabilities[assignment],
                r
            );
        }
    }

    pub fn update_activity(&mut self, index: usize, increase_value: f64) {
        self.activities[index] = (1.0 - 1.0 / self.time_constant) * self.activities[index]
            + increase_value / self.time_constant;
        if self.unassigned_variables.contains_key(index) {
            self.unassigned_variables
                .insert(index, self.activities[index]);
        }
    }

    // pub fn update_conflict_probabilities(
    //     &mut self,
    //     conflict_assignments: impl Iterator<Item = Literal>,
    // ) {
    // self.activity_increase_value /= 1.0 - 1.0 / self.time_constant;
    // if self.activity_increase_value > 1e4 {
    //     let buffer = Vec::from_iter(
    //         self.unassigned_variables
    //             .iter()
    //             .map(|(&index, &activity)| (index, activity)),
    //     );
    //     self.unassigned_variables.clear();
    //     for (index, activity) in buffer.into_iter() {
    //         debug_assert!(activity == self.activities[index]);
    //         self.unassigned_variables
    //             .insert(index, activity / self.activity_increase_value);
    //     }
    //     for activity in self.activities.iter_mut() {
    //         *activity /= self.activity_increase_value;
    //     }
    //     self.activity_increase_value = 1.0;
    // }

    // for assignment in conflict_assignments {
    //     // self.activities[assignment.index()] += self.activity_increase_value;
    //     self.unassigned_variables
    //         .insert(assignment.index(), self.activities[assignment.index()]);
    // }
    // }

    pub fn assignment_probability(&self, literal: Literal) -> f64 {
        return self.assignment_probabilities[literal];
    }

    pub fn activity(&self, index: usize) -> f64 {
        // return self.activities[index] / self.activity_increase_value;
        return self.activities[index];
    }

    pub fn push_unassigned_variable(&mut self, index: usize) {
        self.unassigned_variables
            .insert(index, self.activities[index]);
    }

    pub fn pop_unassigned_variable(&mut self) -> Option<usize> {
        return self
            .unassigned_variables
            .pop_first()
            .map(|(index, _)| index);
    }
}

#[derive(Default, Clone)]
struct CompareUnassignedVariables {}

impl FnOnce<(&(usize, f64), &(usize, f64))> for CompareUnassignedVariables {
    type Output = std::cmp::Ordering;
    extern "rust-call" fn call_once(
        self,
        (lhs, rhs): (&(usize, f64), &(usize, f64)),
    ) -> Self::Output {
        rhs.1.partial_cmp(&lhs.1).unwrap()
    }
}

impl FnMut<(&(usize, f64), &(usize, f64))> for CompareUnassignedVariables {
    extern "rust-call" fn call_mut(
        &mut self,
        (lhs, rhs): (&(usize, f64), &(usize, f64)),
    ) -> Self::Output {
        rhs.1.partial_cmp(&lhs.1).unwrap()
    }
}

impl Fn<(&(usize, f64), &(usize, f64))> for CompareUnassignedVariables {
    extern "rust-call" fn call(&self, (lhs, rhs): (&(usize, f64), &(usize, f64))) -> Self::Output {
        rhs.1.partial_cmp(&lhs.1).unwrap()
    }
}
