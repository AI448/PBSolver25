mod activities;
mod assignment_queue;
mod reason;

use crate::{
    CountConstraint, CountConstraintTrait, LinearConstraint, LinearConstraintTrait, MonadicClause,
    decision_stack::DecisionStack,
    theories::{
        CountConstraintExplainKey, CountConstraintTheory, IntegerLinearConstraintExplainKey,
        IntegerLinearConstraintTheory, MonadicClauseExplainKey, MonadicClauseTheory,
        TheoryAddConstraintTrait, TheoryTrait,
    },
    types::{Boolean, Literal},
};
use activities::Activities;
use assignment_queue::AssignmentQueue;
use either::Either;
use std::ops::Deref;

pub use reason::Reason;

pub enum PBConstraint<
    CountConstraintT = CountConstraint,
    IntegerLinearConstraintT = LinearConstraint<u64>,
> where
    CountConstraintT: CountConstraintTrait,
    IntegerLinearConstraintT: LinearConstraintTrait<Value = u64>,
{
    MonadicClause(MonadicClause),
    CountConstraint(CountConstraintT),
    IntegerLinearConstraint(IntegerLinearConstraintT),
}

impl<CountConstraintT, IntegerLinearConstraintT> LinearConstraintTrait
    for PBConstraint<CountConstraintT, IntegerLinearConstraintT>
where
    CountConstraintT: CountConstraintTrait,
    IntegerLinearConstraintT: LinearConstraintTrait<Value = u64>,
{
    type Value = u64;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone + '_ {
        return match self {
            Self::MonadicClause(monadic_clause) => {
                Either::Left(Either::Left([(monadic_clause.literal, 1)].into_iter()))
            }
            Self::CountConstraint(count_constraint) => Either::Left(Either::Right(
                count_constraint.iter_terms().map(|literal| (literal, 1)),
            )),
            Self::IntegerLinearConstraint(integer_linear_constraint) => {
                Either::Right(integer_linear_constraint.iter_terms())
            }
        };
    }

    fn lower(&self) -> Self::Value {
        return match self {
            Self::MonadicClause(_) => 1,
            Self::CountConstraint(count_constraint) => count_constraint.lower(),
            Self::IntegerLinearConstraint(integer_linear_constraint) => {
                integer_linear_constraint.lower()
            }
        };
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PBExplainKey {
    MonadicClause(MonadicClauseExplainKey),
    CountConstraint(CountConstraintExplainKey),
    IntegerLinearConstraint(IntegerLinearConstraintExplainKey),
}

impl From<MonadicClauseExplainKey> for PBExplainKey {
    fn from(explain_key: MonadicClauseExplainKey) -> Self {
        Self::MonadicClause(explain_key)
    }
}

impl From<CountConstraintExplainKey> for PBExplainKey {
    fn from(explain_key: CountConstraintExplainKey) -> Self {
        Self::CountConstraint(explain_key)
    }
}

impl From<IntegerLinearConstraintExplainKey> for PBExplainKey {
    fn from(explain_key: IntegerLinearConstraintExplainKey) -> Self {
        Self::IntegerLinearConstraint(explain_key)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PBState {
    Noconflict,
    Conflict {
        index: usize,
        explain_keys: [PBExplainKey; 2],
    },
}

impl PBState {
    pub fn is_noconflict(&self) -> bool {
        return matches!(self, Self::Noconflict);
    }

    pub fn is_conflict(&self) -> bool {
        return matches!(self, Self::Conflict { .. });
    }
}

pub struct PBEngine {
    decision_stack: DecisionStack<PBExplainKey>,
    activities: Activities,
    monadic_clause_theory: MonadicClauseTheory,
    count_constraint_theory: CountConstraintTheory,
    integer_linear_constraint_theory: IntegerLinearConstraintTheory,
    assignment_queue: AssignmentQueue<PBExplainKey>,
    state: PBState,
}

impl Deref for PBEngine {
    type Target = DecisionStack<PBExplainKey>;
    fn deref(&self) -> &Self::Target {
        return &self.decision_stack;
    }
}

impl PBEngine {
    pub fn new(activity_time_constant: f64) -> Self {
        Self {
            decision_stack: DecisionStack::default(),
            activities: Activities::new(activity_time_constant),
            monadic_clause_theory: MonadicClauseTheory::new(),
            count_constraint_theory: CountConstraintTheory::new(),
            integer_linear_constraint_theory: IntegerLinearConstraintTheory::new(),
            assignment_queue: AssignmentQueue::default(),
            state: PBState::Noconflict,
        }
    }
    pub fn state(&self) -> PBState {
        return self.state;
    }

    pub fn update_assignment_probabilities(&mut self) {
        self.activities.update_assignment_probabilities(
            (0..self.decision_stack.number_of_assignments())
                .map(|order| self.decision_stack.get_assignment(order)),
        );
    }

    pub fn update_conflict_probabilities(
        &mut self,
        conflict_assignments: impl Iterator<Item = Literal>,
    ) {
        self.activities
            .update_conflict_probabilities(conflict_assignments);
    }

    pub fn assignment_probability(&self, literal: Literal) -> f64 {
        return self.activities.assignment_probability(literal);
    }

    pub fn conflict_probability(&self, literal: Literal) -> f64 {
        return self.activities.conflict_probability(literal);
    }

    pub fn activity(&self, index: usize) -> f64 {
        return self.activities.activity(index);
    }

    pub fn number_of_monadic_clauses(&self) -> usize {
        return self.monadic_clause_theory.number_of_monadic_clauses();
    }

    pub fn number_of_count_constraints(&self) -> usize {
        return self.count_constraint_theory.number_of_constraints();
    }

    pub fn number_of_integer_linear_constraints(&self) -> usize {
        return self.integer_linear_constraint_theory.number_of_constraints();
    }

    pub fn add_variable_with_initial_value(&mut self, initial_value: Boolean) {
        self.decision_stack.add_variable(initial_value);
        self.activities.add_variable();
        self.monadic_clause_theory.add_variable();
        self.count_constraint_theory.add_variable();
        self.integer_linear_constraint_theory.add_variable();
    }

    pub fn add_constraint(
        &mut self,
        constraint: PBConstraint<
            impl CountConstraintTrait,
            impl LinearConstraintTrait<Value = u64>,
        >,
    ) {
        assert!(self.state.is_noconflict()); // TODO: あとで他の状態にも対応する
        match constraint {
            PBConstraint::MonadicClause(constraint) => {
                self.monadic_clause_theory
                    .add_constraint(constraint, &self.decision_stack, |propagation| {
                        self.assignment_queue.push(
                            propagation.literal,
                            Reason::Propagation {
                                explain_key: propagation.explain_key.into(),
                            },
                            0,
                        );
                    })
                    .unwrap();
            }
            PBConstraint::CountConstraint(constraint) => {
                self.count_constraint_theory
                    .add_constraint(constraint, &self.decision_stack, |propagation| {
                        self.assignment_queue.push(
                            propagation.literal,
                            Reason::Propagation {
                                explain_key: propagation.explain_key.into(),
                            },
                            0,
                        );
                    })
                    .unwrap();
            }
            PBConstraint::IntegerLinearConstraint(constraint) => {
                self.integer_linear_constraint_theory
                    .add_constraint(constraint, &self.decision_stack, |propagation| {
                        self.assignment_queue.push(
                            propagation.literal,
                            Reason::Propagation {
                                explain_key: propagation.explain_key.into(),
                            },
                            0,
                        )
                    })
                    .unwrap();
            }
        }
        // self.propagate();
        // return self.state();
    }

    pub fn decide(&mut self, literal: Literal) {
        assert!(self.state.is_noconflict());
        debug_assert!(self.assignment_queue.is_empty());
        self.assignment_queue.push(literal, Reason::Decision, 1);
        // self.propagate();
        // return self.state();
    }

    pub fn backjump(&mut self, backjump_level: usize) -> PBState {
        assert!(backjump_level < self.decision_stack.decision_level());
        self.integer_linear_constraint_theory.backjump(backjump_level, &self.decision_stack);
        self.count_constraint_theory
            .backjump(backjump_level, &self.decision_stack);
        self.monadic_clause_theory
            .backjump(backjump_level, &self.decision_stack);
        self.decision_stack.backjump(backjump_level);
        self.state = PBState::Noconflict;
        return self.state();
    }

    pub fn explain(
        &self,
        explain_key: PBExplainKey,
    ) -> PBConstraint<impl CountConstraintTrait + '_, impl LinearConstraintTrait<Value = u64> + '_>
    {
        return match explain_key {
            PBExplainKey::MonadicClause(explain_key) => {
                PBConstraint::MonadicClause(self.monadic_clause_theory.explain(explain_key))
            }
            PBExplainKey::CountConstraint(explain_key) => {
                PBConstraint::CountConstraint(self.count_constraint_theory.explain(explain_key))
            }
            PBExplainKey::IntegerLinearConstraint(explain_key) => {
                PBConstraint::IntegerLinearConstraint(
                    self.integer_linear_constraint_theory.explain(explain_key),
                )
            }
        };
    }

    pub fn propagate(&mut self) -> PBState {
        debug_assert!(self.state.is_noconflict());
        loop {
            if let Some((index, reasons)) = self.assignment_queue.pop_conflict() {
                let explain_keys = reasons.map(|reason| {
                    if let Reason::Propagation { explain_key } = reason {
                        explain_key
                    } else {
                        unreachable!()
                    }
                });
                self.state = PBState::Conflict {
                    index,
                    explain_keys,
                };
                break;
            }
            let Some((literal, reason)) = self.assignment_queue.pop_assignment() else {
                break;
            };

            self.decision_stack.assign(literal, reason);

            self.monadic_clause_theory
                .assign(&self.decision_stack, |propagation| {
                    self.assignment_queue.push(
                        propagation.literal,
                        Reason::Propagation {
                            explain_key: propagation.explain_key.into(),
                        },
                        0,
                    );
                });
            self.count_constraint_theory
                .assign(&self.decision_stack, |propagation| {
                    self.assignment_queue.push(
                        propagation.literal,
                        Reason::Propagation {
                            explain_key: propagation.explain_key.into(),
                        },
                        0,
                    );
                });
            self.integer_linear_constraint_theory
                .assign(&self.decision_stack, |propagation| {
                    self.assignment_queue.push(
                        propagation.literal,
                        Reason::Propagation {
                            explain_key: propagation.explain_key.into(),
                        },
                        0,
                    )
                });
        }
        self.assignment_queue.clear();

        return self.state;
    }
}
