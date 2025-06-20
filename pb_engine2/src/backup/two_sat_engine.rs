use std::collections::VecDeque;

use either::Either;

use crate::{
    collections::LiteralArray, core_engine::{CoreEngine, MonadicClause}, decision_stack::DecisionStack, etc::{CompositeConstraint, CompositeExplainKey, Reason, State}, Literal
};

pub trait CliqueConstraintTrait {
    fn iter_literals(&self) -> impl Iterator<Item = Literal>;
}

#[derive(Clone)]
pub struct CliqueConstraint {
    literals: Vec<Literal>,
}

impl CliqueConstraint {
    pub fn new(literals: impl Iterator<Item = Literal>) -> Self {
        return Self {
            literals: literals.collect(),
        };
    }
}

impl CliqueConstraintTrait for CliqueConstraint {
    fn iter_literals(&self) -> impl Iterator<Item = Literal> {
        self.literals.iter().cloned()
    }
}

#[derive(Clone)]
pub struct CliqueConstraintView<IteratorT: Iterator<Item = Literal> + Clone> {
    iterator: IteratorT,
}

impl<IteratorT: Iterator<Item = Literal> + Clone> CliqueConstraintView<IteratorT> {
    pub fn new(iterator: IteratorT) -> Self {
        Self { iterator }
    }
}

impl<IteratorT: Iterator<Item = Literal> + Clone> CliqueConstraintTrait
    for CliqueConstraintView<IteratorT>
{
    fn iter_literals(&self) -> impl Iterator<Item = Literal> {
        self.iterator.clone()
    }
}

// pub enum TwoSatConstraint<CliqueT: CliqueConstraintTrait> {
//     MonadicClause(MonadicClause),
//     Clique(CliqueT),
// }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CliqueExplainKey {
    row_id: usize,
}

// #[derive(Clone, Copy, PartialEq, Eq, Debug)]
// pub enum TwoSatExplainKey {
//     MonadicClause{literal: Literal},
//     Clique{row_id: usize},
// }

pub struct TwoSatEngine {
    state: State<CliqueExplainKey>,
    core_engine: CoreEngine,
    rows: Vec<CliqueRow>,
    columns: LiteralArray<QliqueColumn>,
    number_of_evaluated_assignments: usize,
    constraint_queue: VecDeque<CliqueConstraint>,
}

impl TwoSatEngine {
    pub fn state(&self) -> State<CompositeExplainKey> {
        return self.state.into_composite();
    }

    pub fn explain(&self, explain_key: CompositeExplainKey) -> Either<impl CliqueConstraintTrait, MonadicClause> {
        if let CompositeExplainKey::CliqueConstraint(explain_key) = explain_key {
            return Either::Left(CliqueConstraintView::new(self.rows[explain_key.row_id].literals.iter().cloned()));
        } else {
            return Either::Right(self.core_engine.explain(explain_key));
        }
    }

    pub fn add_variable(&mut self) {
        self.core_engine.add_variable();
        self.columns.push([QliqueColumn::default(), QliqueColumn::default()]);
    }

    pub fn add_monadic_clause(&mut self, constraint: MonadicClause) {
        self.core_engine.add_monadic_clause(constraint);
    }

    pub fn add_clique_constraint(&mut self, constraint: impl CliqueConstraintTrait) {
        self.constraint_queue
            .push_back(CliqueConstraint::new(constraint.iter_literals()));
        let min_falsified_decision_level = self
            .constraint_queue
            .back()
            .unwrap()
            .iter_literals()
            .filter(|&literal| self.core_engine.is_false(literal))
            .map(|literal| self.core_engine.get_decision_level(literal.index()))
            .min();

        if let Some(min_falsified_decision_level) = min_falsified_decision_level
            && min_falsified_decision_level < self.core_engine.decision_level()
        {
            self.state = State::BackjumpRequired {
                backjump_level: min_falsified_decision_level,
            };
        } else {
            self.propagate();
        }
    }

    pub fn assign(&mut self, literal: Literal, reason: Reason<CompositeExplainKey>) {
        #[cfg(debug_assertions)]
        if let Reason::Propagation { explain_key } = reason {
            debug_assert!(!matches!(
                explain_key,
                CompositeExplainKey::CliqueConstraint(..)
            ));
        }
        assert!(self.state.is_noconflict());

        self.core_engine.assign(literal, reason);
        self.propagate();
    }

    pub fn backjump(&mut self, backjump_level: usize) {
        // engine の状態をバックジャンプ後の状態に戻す
        let backjump_order = self.core_engine.order_range(backjump_level).end;
        debug_assert!(backjump_order <= self.number_of_evaluated_assignments);
        self.number_of_evaluated_assignments = backjump_order;
        if let State::BackjumpRequired {
            backjump_level: required_backjump_level,
        } = self.state
            && backjump_level <= required_backjump_level
        {
            self.state = State::Noconflict;
        }
        // core_engine のバックジャンプ
        self.core_engine.backjump(backjump_level);
        // 伝播
        self.propagate();
    }

    fn propagate(&mut self) {
        loop {
            while self.number_of_evaluated_assignments < self.core_engine.number_of_assignments() {
                if !self.core_engine.state().is_noconflict() || !self.state.is_noconflict() {
                    return;
                }
                self.propagate_by_assignment();
            }
            if !self.core_engine.state().is_noconflict()
                || !self.state.is_noconflict()
                || self.constraint_queue.is_empty()
            {
                return;
            }
            self.propagate_by_constraint_addition();
        }
    }

    fn propagate_by_assignment(&mut self) {
        debug_assert!(self.core_engine.state().is_noconflict());
        debug_assert!(self.state.is_noconflict());
        debug_assert!(
            self.number_of_evaluated_assignments < self.core_engine.number_of_assignments()
        );

        let assignment = self
            .core_engine
            .get_assignment(self.number_of_evaluated_assignments);
        self.number_of_evaluated_assignments += 1;

        for &row_id in self.columns[!assignment].row_ids.iter() {
            let row = &self.rows[row_id];
            debug_assert!(
                row.literals
                    .iter()
                    .find(|&&literal| literal == !assignment)
                    .is_some()
            );
            let explain_key = CliqueExplainKey { row_id };
            for &literal in row.literals.iter() {
                if literal == !assignment {
                    continue;
                }
                if !self.core_engine.is_assigned(literal.index()) {
                    self.core_engine.assign(
                        literal,
                        Reason::Propagation {
                            explain_key: explain_key.into(),
                        },
                    );
                    if !self.core_engine.state().is_noconflict() {
                        return;
                    }
                } else if self.core_engine.is_false(literal) {
                    self.state = State::Conflict { explain_key };
                    return;
                }
            }
        }
    }

    fn propagate_by_constraint_addition(&mut self) {
        debug_assert!(self.core_engine.state().is_noconflict());
        debug_assert!(self.state.is_noconflict());
        debug_assert!(
            self.number_of_evaluated_assignments == self.core_engine.number_of_assignments()
        );
        debug_assert!(!self.constraint_queue.is_empty());

        let constraint = self.constraint_queue.pop_front().unwrap();
        debug_assert!(
            !constraint
                .iter_literals()
                .any(|literal| self.core_engine.is_false(literal)
                    && self.core_engine.get_decision_level(literal.index())
                        < self.core_engine.decision_level(),),
            "Should not exist any literals that assigned False before current decision level."
        );

        let row_id = self.rows.len();
        self.rows.push(CliqueRow {
            literals: constraint.literals,
        });
        let row = self.rows.last().unwrap();

        for &literal in row.literals.iter() {
            self.columns[literal].row_ids.push(row_id);
        }

        let explain_key = CliqueExplainKey { row_id };

        let number_of_falsified_literals = row
            .literals
            .iter()
            .filter(|&&literal| self.core_engine.is_false(literal))
            .count();
        if number_of_falsified_literals == 1 {
            let &falsified_literal = row
                .literals
                .iter()
                .find(|&&literal| self.core_engine.is_false(literal))
                .unwrap();
            for &literal in row.literals.iter() {
                if literal == falsified_literal {
                    continue;
                }
                if !self.core_engine.is_assigned(literal.index()) {
                    self.core_engine.assign(
                        literal,
                        Reason::Propagation {
                            explain_key: explain_key.into(),
                        },
                    );
                    if !self.core_engine.state().is_noconflict() {
                        return;
                    }
                } else if self.core_engine.is_false(literal) {
                    self.state = State::Conflict { explain_key };
                    return;
                }
            }
        } else if number_of_falsified_literals >= 2 {
            self.state = State::Conflict { explain_key };
            return;
        }
    }
}


pub trait Explain<ExplainKey> {
    type ExplanationConstraint<'a> where Self: 'a;
    fn explain(&self, explain_key: ExplainKey) -> Self::ExplanationConstraint<'_>;
}


impl  Explain<CliqueExplainKey> for TwoSatEngine {
    type ExplanationConstraint<'a> = impl CliqueConstraintTrait + 'a;
    fn explain(&self, explain_key: CliqueExplainKey) -> Self::ExplanationConstraint<'_> {
        return CliqueConstraintView::new(self.rows[explain_key.row_id].literals.iter().cloned());
    }
}

impl<ExplainKeyT> Explain<ExplainKeyT> for TwoSatEngine
where
    CoreEngine: Explain<ExplainKeyT>
{
    type ExplanationConstraint<'a> = <CoreEngine as Explain<ExplainKeyT>>::ExplanationConstraint<'a>;
    fn explain(&self, explain_key: ExplainKeyT) -> Self::ExplanationConstraint<'_> {
        return self.core_engine.explain(explain_key);
    }
}


#[derive(Clone, Debug)]
struct CliqueRow {
    literals: Vec<Literal>,
}

#[derive(Default, Clone, Debug)]
struct QliqueColumn {
    row_ids: Vec<usize>,
}
