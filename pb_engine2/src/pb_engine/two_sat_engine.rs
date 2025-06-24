use super::etc::State;
use crate::{
    ConstraintView, LinearConstraintTrait, Literal,
    collections::LiteralArray,
    constraint::UnsignedIntegerTrait,
    pb_engine::{DecisionStack, OneSatEngine, OneSatEngineExplainKey, Reason},
};
use either::Either;
use num::{One, ToPrimitive};
use std::{collections::VecDeque, ops::Deref};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CliqueConstraintExplainKey {
    row_id: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TwoSatEngineExplainKey {
    CliqueConstraint(CliqueConstraintExplainKey),
    OneSatEngine(OneSatEngineExplainKey),
}

impl From<CliqueConstraintExplainKey> for TwoSatEngineExplainKey {
    fn from(explain_key: CliqueConstraintExplainKey) -> Self {
        Self::CliqueConstraint(explain_key)
    }
}

impl<ExplainKeyT> From<ExplainKeyT> for TwoSatEngineExplainKey
where
    ExplainKeyT: Into<OneSatEngineExplainKey>,
{
    fn from(explain_key: ExplainKeyT) -> Self {
        Self::OneSatEngine(explain_key.into())
    }
}

#[derive(Clone, Debug)]
struct CliqueConstraint {
    literals: Vec<Literal>,
}

#[derive(Clone, Debug)]
struct Row {
    constraint: CliqueConstraint,
    is_learnt: bool,
}

#[derive(Default, Clone, Debug)]
struct Column {
    row_ids: Vec<usize>,
}

pub struct TwoSatEngine<CompositeExplainKeyT> {
    state: State<CliqueConstraintExplainKey>,
    inner_engine: OneSatEngine<CompositeExplainKeyT>,
    rows: Vec<Row>,
    columns: LiteralArray<Column>,
    number_of_confirmed_assignments: usize,
    constraint_queue: VecDeque<(CliqueConstraint, bool)>,
}

impl<CompositeExplainKeyT> TwoSatEngine<CompositeExplainKeyT> {
    pub fn new() -> Self {
        Self {
            state: State::Noconflict,
            inner_engine: OneSatEngine::new(),
            rows: Vec::default(),
            columns: LiteralArray::default(),
            number_of_confirmed_assignments: 0,
            constraint_queue: VecDeque::default(),
        }
    }
}

impl<CompositeExplainKeyT> Deref for TwoSatEngine<CompositeExplainKeyT> {
    type Target = DecisionStack<CompositeExplainKeyT>;
    fn deref(&self) -> &Self::Target {
        self.inner_engine.deref()
    }
}

impl<CompositeExplainKeyT> TwoSatEngine<CompositeExplainKeyT>
where
    CompositeExplainKeyT: From<OneSatEngineExplainKey> + From<CliqueConstraintExplainKey>,
{
    pub fn state(&self) -> State<TwoSatEngineExplainKey> {
        return self.state.composite(self.inner_engine.state());
    }

    pub fn explain<ValueT>(
        &self,
        explain_key: TwoSatEngineExplainKey,
    ) -> impl LinearConstraintTrait<Value = ValueT>
    where
        ValueT: UnsignedIntegerTrait,
    {
        return match explain_key {
            TwoSatEngineExplainKey::CliqueConstraint(explain_key) => {
                let row = &self.rows[explain_key.row_id];
                Either::Left(ConstraintView::new(
                    row.constraint.literals.iter().map(|&literal| (literal, ValueT::one())),
                    ValueT::from_usize(row.constraint.literals.len() - 1).unwrap(),
                ))
            }
            TwoSatEngineExplainKey::OneSatEngine(explain_key) => {
                Either::Right(self.inner_engine.explain(explain_key))
            }
        };
    }

    pub fn add_variable(&mut self) {
        self.inner_engine.add_variable();
        self.columns.push([Column::default(), Column::default()]);
    }

    pub fn assign(&mut self, literal: Literal, reason: Reason<CompositeExplainKeyT>) {
        assert!(self.state.is_noconflict());
        assert!(self.inner_engine.state().is_noconflict());
        debug_assert!(
            self.number_of_confirmed_assignments == self.inner_engine.number_of_assignments()
        );

        self.inner_engine.assign(literal, reason);
        self.propagate();
    }

    pub fn backjump(&mut self, backjump_level: usize) {
        let backjump_order = self.inner_engine.order_range(backjump_level).end;
        debug_assert!(backjump_order <= self.number_of_confirmed_assignments);
        self.number_of_confirmed_assignments = backjump_order;
        self.inner_engine.backjump(backjump_level);
        if self.state.is_conflict()
            || self.state.is_backjump_required_and(|required_backjump_level| {
                backjump_level <= required_backjump_level
            })
        {
            self.state = State::Noconflict;
        }
        self.propagate();
    }

    pub fn add_constraint<LinearConstraintT>(
        &mut self,
        constraint: &LinearConstraintT,
        is_learnt: bool,
    ) where
        LinearConstraintT: LinearConstraintTrait,
    {
        assert!(
            constraint
                .iter_terms()
                .all(|(_, coefficient)| coefficient == LinearConstraintT::Value::one())
        );
        assert!(constraint.iter_terms().count() <= constraint.lower().to_usize().unwrap() + 1);

        if constraint.iter_terms().count() <= constraint.lower().to_usize().unwrap() {
            self.inner_engine.add_constraint(constraint, is_learnt);
        } else {
            // CliqueConstraint を構築
            let constraint = CliqueConstraint {
                literals: Vec::from_iter(constraint.iter_terms().map(|(literal, _)| literal)),
            };

            // False が割り当てられているリテラルが存在すればその最小の決定レベルを取得
            let min_falsified_decision_level = constraint
                .literals
                .iter()
                .filter(|&&literal| self.inner_engine.is_false(literal))
                .map(|&literal| self.inner_engine.get_decision_level(literal.index()))
                .min();

            // 現在の決定レベルよりも前に伝播が発生するなら state を BackjumpRequired に
            if let Some(min_falsified_decision_level) = min_falsified_decision_level
                && min_falsified_decision_level < self.inner_engine.decision_level()
            {
                self.state.merge(State::BackjumpRequired {
                    backjump_level: min_falsified_decision_level,
                });
                debug_assert!(self.state.is_backjump_required());
            }

            // 制約条件をキューに追加
            self.constraint_queue.push_back((constraint, is_learnt));
        }

        self.propagate();
    }

    fn propagate(&mut self) {
        while self.state.is_noconflict() && self.inner_engine.state().is_noconflict() {
            if self.number_of_confirmed_assignments < self.inner_engine.number_of_assignments() {
                self.propagate_by_assignment();
            } else if !self.constraint_queue.is_empty() {
                self.propagate_by_constraint_addition();
            } else {
                break;
            }
        }
    }

    fn propagate_by_assignment(&mut self) {
        debug_assert!(self.inner_engine.state().is_noconflict());
        debug_assert!(self.state.is_noconflict());
        debug_assert!(
            self.number_of_confirmed_assignments < self.inner_engine.number_of_assignments()
        );

        let assignment = self.inner_engine.get_assignment(self.number_of_confirmed_assignments);
        self.number_of_confirmed_assignments += 1;

        for &row_id in self.columns[!assignment].row_ids.iter() {
            let row = &self.rows[row_id];
            debug_assert!(
                row.constraint.literals.iter().find(|&&literal| literal == !assignment).is_some()
            );
            let explain_key = CliqueConstraintExplainKey { row_id };
            for &literal in row.constraint.literals.iter() {
                if literal == !assignment {
                    continue;
                }
                // 未割り当てである場合
                if !self.inner_engine.is_assigned(literal.index()) {
                    // 伝播
                    self.inner_engine.assign(
                        literal,
                        Reason::Propagation {
                            explain_key: explain_key.into(),
                        },
                    );
                    if !self.inner_engine.state().is_noconflict() {
                        return;
                    }
                // すでに False が割り当てられている場合
                } else if self.inner_engine.is_false(literal) {
                    // 現在の決定レベルより前に False が割り当てられていることはないはず
                    debug_assert!(
                        self.inner_engine.get_decision_level(literal.index())
                            == self.inner_engine.decision_level()
                    );
                    // state を Conflict にして return
                    self.state.merge(State::Conflict { explain_key });
                    return;
                }
            }
        }
    }

    fn propagate_by_constraint_addition(&mut self) {
        debug_assert!(self.state.is_noconflict());
        debug_assert!(self.inner_engine.state().is_noconflict());
        debug_assert!(
            self.number_of_confirmed_assignments == self.inner_engine.number_of_assignments()
        );
        debug_assert!(!self.constraint_queue.is_empty());

        // キューから制約を取り出し
        let (constraint, is_learnt) = self.constraint_queue.pop_front().unwrap();

        // 現在の決定レベルよりも前に False が割り当てられたリテラルは存在しないはず
        debug_assert!(!constraint.literals.iter().any(
            |&literal| self.inner_engine.is_false(literal)
                && self.inner_engine.get_decision_level(literal.index())
                    < self.inner_engine.decision_level()
        ));

        // 行を追加
        let row_id = self.rows.len();
        let explain_key = CliqueConstraintExplainKey { row_id };
        self.rows.push(Row {
            constraint,
            is_learnt,
        });
        let row = self.rows.last().unwrap();

        // リテラルを含む列に row_id を追加
        for &literal in row.constraint.literals.iter() {
            self.columns[literal].row_ids.push(row_id);
        }

        // false が割り当てられているリテラルの数を算出
        let number_of_falsified_literals = row
            .constraint
            .literals
            .iter()
            .filter(|&&literal| self.inner_engine.is_false(literal))
            .count();

        if number_of_falsified_literals == 1 {
            // False が割り当てられているリテラルがちょうど 1 つであれば伝播
            let &falsified_literal = row
                .constraint
                .literals
                .iter()
                .find(|&&literal| self.inner_engine.is_false(literal))
                .unwrap();
            for &literal in row.constraint.literals.iter() {
                if literal == falsified_literal {
                    continue;
                }
                // 未割り当てである場合
                if !self.inner_engine.is_assigned(literal.index()) {
                    self.inner_engine.assign(
                        literal,
                        Reason::Propagation {
                            explain_key: explain_key.into(),
                        },
                    );
                    if !self.inner_engine.state().is_noconflict() {
                        return;
                    }
                // すでに False が割り当てられている場合
                } else if self.inner_engine.is_false(literal) {
                    self.state = State::Conflict { explain_key };
                    return;
                }
            }
        } else if number_of_falsified_literals >= 2 {
            // 矛盾している場合には state を Conflict に
            self.state.merge(State::Conflict { explain_key });
            return;
        }
    }
}
