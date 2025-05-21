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
            count_constraint_theory: CountConstraintTheory::new(1e4),
            integer_linear_constraint_theory: IntegerLinearConstraintTheory::new(1e4),
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
        return self
            .integer_linear_constraint_theory
            .number_of_constraints();
    }

    pub fn add_variable_with_initial_value(&mut self, initial_value: Boolean) {
        self.decision_stack.add_variable(initial_value);
        self.activities.add_variable();
        self.monadic_clause_theory.add_variable();
        self.count_constraint_theory.add_variable();
        self.integer_linear_constraint_theory.add_variable();
    }

    pub fn add_monadic_clause(&mut self, monadic_clause: MonadicClause, is_learnt: bool) {
        Self::add_constraint_to(
            &mut self.monadic_clause_theory,
            monadic_clause,
            is_learnt,
            &self.decision_stack,
            &mut self.assignment_queue,
            &self.activities,
        );
    }

    pub fn add_count_constraint(
        &mut self,
        count_constraint: impl CountConstraintTrait,
        is_learnt: bool,
    ) {
        Self::add_constraint_to(
            &mut self.count_constraint_theory,
            count_constraint,
            is_learnt,
            &self.decision_stack,
            &mut self.assignment_queue,
            &self.activities,
        );
    }

    pub fn add_integer_linear_constraint(
        &mut self,
        constraint: impl LinearConstraintTrait<Value = u64>,
        is_learnt: bool,
    ) {
        Self::add_constraint_to(
            &mut self.integer_linear_constraint_theory,
            constraint,
            is_learnt,
            &self.decision_stack,
            &mut self.assignment_queue,
            &self.activities,
        );
    }

    fn add_constraint_to<TheoryT, ConstraintT>(
        theory: &mut TheoryT,
        constraint: ConstraintT,
        is_learnt: bool,
        decision_stack: &DecisionStack<impl Copy>,
        assignment_queue: &mut AssignmentQueue<PBExplainKey>,
        activities: &Activities,
    ) where
        TheoryT: TheoryAddConstraintTrait<ConstraintT>,
        TheoryT::ExplainKey: Into<PBExplainKey>,
    {
        theory
            .add_constraint(constraint, is_learnt, decision_stack, |propagation| {
                assignment_queue.push(
                    propagation.literal,
                    Reason::Propagation {
                        explain_key: propagation.explain_key.into(),
                    },
                    activities.activity(propagation.literal.index()),
                    propagation.plbd,
                );
            })
            .unwrap();
    }

    pub fn decide(&mut self) {
        assert!(self.state.is_noconflict());
        debug_assert!(self.assignment_queue.is_empty());
        let decision_variable = {
            let mut decision_variable = None;
            loop {
                let variable = self.activities.pop_unassigned_variable().unwrap();
                if !self.decision_stack.is_assigned(variable) {
                    decision_variable.replace(variable);
                    break;
                }
            }
            decision_variable.unwrap()
        };
        let decision_value = self.decision_stack.get_value(decision_variable);
        self.assignment_queue.push(
            Literal::new(decision_variable, decision_value),
            Reason::Decision,
            f64::INFINITY,
            0,
        );
        // self.propagate();
        // return self.state();
    }

    pub fn backjump(&mut self, backjump_level: usize) -> PBState {
        assert!(backjump_level < self.decision_stack.decision_level());
        self.integer_linear_constraint_theory
            .backjump(backjump_level, &self.decision_stack);
        self.count_constraint_theory
            .backjump(backjump_level, &self.decision_stack);
        self.monadic_clause_theory
            .backjump(backjump_level, &self.decision_stack);

        for order in self.decision_stack.order_range(backjump_level).end
            ..self.decision_stack.number_of_assignments()
        {
            self.activities
                .push_unassigned_variable(self.decision_stack.get_assignment(order).index());
        }

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
                        self.activities.activity(propagation.literal.index()),
                        propagation.plbd,
                    );
                });
            self.count_constraint_theory
                .assign(&self.decision_stack, |propagation| {
                    self.assignment_queue.push(
                        propagation.literal,
                        Reason::Propagation {
                            explain_key: propagation.explain_key.into(),
                        },
                        self.activities.activity(propagation.literal.index()),
                        propagation.plbd,
                    );
                });
            self.integer_linear_constraint_theory
                .assign(&self.decision_stack, |propagation| {
                    self.assignment_queue.push(
                        propagation.literal,
                        Reason::Propagation {
                            explain_key: propagation.explain_key.into(),
                        },
                        self.activities.activity(propagation.literal.index()),
                        propagation.plbd,
                    )
                });
        }
        self.assignment_queue.clear();

        return self.state;
    }
}
