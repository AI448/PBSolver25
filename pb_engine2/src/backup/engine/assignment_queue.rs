use std::cmp::Ordering;

use utility::HeapedMap;

use crate::types::{Boolean, Literal};

use super::Reason;

#[derive(Clone)]
pub struct AssignmentQueue<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    conflict_queue: HeapedMap<Conflict<ExplainKeyT, f64>, ConflictComparator>,
    assignment_queue: HeapedMap<Assignment<ExplainKeyT, f64>, AssignmentComparator>,
}

impl<ExplainKeyT> Default for AssignmentQueue<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    fn default() -> Self {
        Self {
            conflict_queue: HeapedMap::default(),
            assignment_queue: HeapedMap::default(),
        }
    }
}

impl<ExplainKeyT> AssignmentQueue<ExplainKeyT>
where
    ExplainKeyT: Copy,
{
    pub fn is_empty(&self) -> bool {
        return self.conflict_queue.is_empty() && self.assignment_queue.is_empty();
    }

    pub fn push(
        &mut self,
        literal: Literal,
        reason: Reason<ExplainKeyT>,
        priority: f64,
        plbd: usize,
    ) {
        if self.conflict_queue.contains_key(literal.index()) {
            let conflict = self.conflict_queue.get(literal.index()).unwrap();
            if plbd < conflict.plbds[literal.value()] {
                let mut conflict = conflict.clone();
                conflict.reasons[literal.value()] = reason;
                conflict.plbds[literal.value()] = plbd;
                self.conflict_queue.insert(literal.index(), conflict);
            }
        } else {
            if !self.assignment_queue.contains_key(literal.index()) {
                self.assignment_queue.insert(
                    literal.index(),
                    Assignment {
                        value: literal.value(),
                        reason,
                        priority,
                        plbd,
                    },
                );
            } else {
                let assignment = self.assignment_queue.get(literal.index()).unwrap();
                if assignment.value == literal.value() {
                    debug_assert!(assignment.priority == priority);
                    if plbd < assignment.plbd {
                        self.assignment_queue.insert(
                            literal.index(),
                            Assignment {
                                value: literal.value(),
                                reason,
                                priority,
                                plbd,
                            },
                        );
                    }
                } else {
                    let reasons = if literal.value() == Boolean::FALSE {
                        [reason, assignment.reason]
                    } else {
                        [assignment.reason, reason]
                    };
                    let plbds = if literal.value() == Boolean::FALSE {
                        [plbd, assignment.plbd]
                    } else {
                        [assignment.plbd, plbd]
                    };
                    self.conflict_queue.insert(
                        literal.index(),
                        Conflict {
                            reasons,
                            plbds,
                            priority,
                        },
                    );
                    self.assignment_queue.remove(literal.index());
                }
            }
        }
    }

    pub fn pop_conflict(&mut self) -> Option<(usize, [Reason<ExplainKeyT>; 2])> {
        return self
            .conflict_queue
            .pop_first()
            .map(|(index, conflict)| (index, conflict.reasons));
    }

    pub fn pop_assignment(&mut self) -> Option<(Literal, Reason<ExplainKeyT>)> {
        assert!(self.conflict_queue.is_empty());
        return self
            .assignment_queue
            .pop_first()
            .map(|(index, assignment)| (Literal::new(index, assignment.value), assignment.reason));
    }

    pub fn clear(&mut self) {
        self.conflict_queue.clear();
        self.assignment_queue.clear();
    }
}

// TODO: PriorityT は (activity, plbd) とする
// 合流では plbd の小さいものを選択し，割り当て順では activity の大きいものを選択する

#[derive(Clone)]
struct Assignment<ExplainKeyT, PriorityT>
where
    ExplainKeyT: Copy,
    PriorityT: PartialOrd,
{
    value: Boolean,
    reason: Reason<ExplainKeyT>,
    plbd: usize,
    priority: PriorityT,
}

#[derive(Clone)]
struct Conflict<ExplainKeyT, PriorityT>
where
    ExplainKeyT: Copy,
    PriorityT: PartialOrd,
{
    reasons: [Reason<ExplainKeyT>; 2],
    plbds: [usize; 2],
    priority: PriorityT,
}

#[derive(Default, Clone)]
struct AssignmentComparator {}

impl<ExplainKeyT, PriorityT>
    FnOnce<(
        &(usize, Assignment<ExplainKeyT, PriorityT>),
        &(usize, Assignment<ExplainKeyT, PriorityT>),
    )> for AssignmentComparator
where
    ExplainKeyT: Copy,
    PriorityT: PartialOrd,
{
    type Output = Ordering;
    extern "rust-call" fn call_once(
        self,
        (lhs, rhs): (
            &(usize, Assignment<ExplainKeyT, PriorityT>),
            &(usize, Assignment<ExplainKeyT, PriorityT>),
        ),
    ) -> Self::Output {
        rhs.1.priority.partial_cmp(&lhs.1.priority).unwrap()
    }
}
impl<ExplainKeyT, PriorityT>
    FnMut<(
        &(usize, Assignment<ExplainKeyT, PriorityT>),
        &(usize, Assignment<ExplainKeyT, PriorityT>),
    )> for AssignmentComparator
where
    ExplainKeyT: Copy,
    PriorityT: PartialOrd,
{
    extern "rust-call" fn call_mut(
        &mut self,
        (lhs, rhs): (
            &(usize, Assignment<ExplainKeyT, PriorityT>),
            &(usize, Assignment<ExplainKeyT, PriorityT>),
        ),
    ) -> Self::Output {
        rhs.1.priority.partial_cmp(&lhs.1.priority).unwrap()
    }
}
impl<ExplainKeyT, PriorityT>
    Fn<(
        &(usize, Assignment<ExplainKeyT, PriorityT>),
        &(usize, Assignment<ExplainKeyT, PriorityT>),
    )> for AssignmentComparator
where
    ExplainKeyT: Copy,
    PriorityT: PartialOrd,
{
    extern "rust-call" fn call(
        &self,
        (lhs, rhs): (
            &(usize, Assignment<ExplainKeyT, PriorityT>),
            &(usize, Assignment<ExplainKeyT, PriorityT>),
        ),
    ) -> Self::Output {
        rhs.1.priority.partial_cmp(&lhs.1.priority).unwrap()
    }
}

#[derive(Default, Clone)]
struct ConflictComparator {}

impl<ExplainKeyT, PriorityT>
    FnOnce<(
        &(usize, Conflict<ExplainKeyT, PriorityT>),
        &(usize, Conflict<ExplainKeyT, PriorityT>),
    )> for ConflictComparator
where
    ExplainKeyT: Copy,
    PriorityT: PartialOrd,
{
    type Output = Ordering;
    extern "rust-call" fn call_once(
        self,
        (lhs, rhs): (
            &(usize, Conflict<ExplainKeyT, PriorityT>),
            &(usize, Conflict<ExplainKeyT, PriorityT>),
        ),
    ) -> Self::Output {
        rhs.1.priority.partial_cmp(&lhs.1.priority).unwrap()
    }
}
impl<ExplainKeyT, PriorityT>
    FnMut<(
        &(usize, Conflict<ExplainKeyT, PriorityT>),
        &(usize, Conflict<ExplainKeyT, PriorityT>),
    )> for ConflictComparator
where
    ExplainKeyT: Copy,
    PriorityT: PartialOrd,
{
    extern "rust-call" fn call_mut(
        &mut self,
        (lhs, rhs): (
            &(usize, Conflict<ExplainKeyT, PriorityT>),
            &(usize, Conflict<ExplainKeyT, PriorityT>),
        ),
    ) -> Self::Output {
        rhs.1.priority.partial_cmp(&lhs.1.priority).unwrap()
    }
}
impl<ExplainKeyT, PriorityT>
    Fn<(
        &(usize, Conflict<ExplainKeyT, PriorityT>),
        &(usize, Conflict<ExplainKeyT, PriorityT>),
    )> for ConflictComparator
where
    ExplainKeyT: Copy,
    PriorityT: PartialOrd,
{
    extern "rust-call" fn call(
        &self,
        (lhs, rhs): (
            &(usize, Conflict<ExplainKeyT, PriorityT>),
            &(usize, Conflict<ExplainKeyT, PriorityT>),
        ),
    ) -> Self::Output {
        rhs.1.priority.partial_cmp(&lhs.1.priority).unwrap()
    }
}
