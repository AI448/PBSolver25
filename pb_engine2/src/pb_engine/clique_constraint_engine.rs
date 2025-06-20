use std::{collections::VecDeque, ops::Deref};

use either::Either;

use crate::{
    Literal,
    collections::LiteralArray,
    engine_trait::{EngineAddConstraintTrait, EngineTrait},
    etc::{Reason, State},
};

pub trait CliqueConstraintTrait {
    fn iter_literals(&self) -> impl Iterator<Item = Literal> + Clone;
}

#[derive(Clone, Debug)]
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

impl<CliqueConstraintT> From<&CliqueConstraintT> for CliqueConstraint
where
    CliqueConstraintT: CliqueConstraintTrait,
{
    fn from(clique_constraint: &CliqueConstraintT) -> Self {
        return Self {
            literals: clique_constraint.iter_literals().collect(),
        };
    }
}

impl CliqueConstraintTrait for CliqueConstraint {
    fn iter_literals(&self) -> impl Iterator<Item = Literal> + Clone {
        self.literals.iter().cloned()
    }
}

#[derive(Clone, Debug)]
pub struct CliqueConstraintView<IteratorT> {
    iterator: IteratorT,
}

impl<IteratorT> CliqueConstraintView<IteratorT> {
    pub fn new(iterator: IteratorT) -> Self {
        Self { iterator }
    }
}

impl<IteratorT> CliqueConstraintTrait for CliqueConstraintView<IteratorT>
where
    IteratorT: Iterator<Item = Literal> + Clone,
{
    fn iter_literals(&self) -> impl Iterator<Item = Literal> + Clone {
        self.iterator.clone()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CliqueConstraintExplainKey {
    row_id: usize,
}

#[derive(Clone, Debug)]
struct Row {
    literals: Vec<Literal>,
    is_learnt: bool,
}

impl CliqueConstraintTrait for &Row {
    fn iter_literals(&self) -> impl Iterator<Item = Literal> + Clone {
        return self.literals.iter().cloned();
    }
}

#[derive(Default, Clone, Debug)]
struct Column {
    row_ids: Vec<usize>,
}

pub struct CliqueConstraintEngine<InnerEngineT> {
    state: State<CliqueConstraintExplainKey>,
    inner_engine: InnerEngineT,
    rows: Vec<Row>,
    columns: LiteralArray<Column>,
    number_of_confirmed_assignments: usize,
    constraint_queue: VecDeque<(CliqueConstraint, bool)>,
}

impl<InnerEngineT> CliqueConstraintEngine<InnerEngineT> {
    pub fn new(inner_engine: InnerEngineT) -> Self {
        Self {
            state: State::Noconflict,
            inner_engine,
            rows: Vec::default(),
            columns: LiteralArray::default(),
            number_of_confirmed_assignments: 0,
            constraint_queue: VecDeque::default(),
        }
    }
}

impl<InnerEngineT> Deref for CliqueConstraintEngine<InnerEngineT>
where
    InnerEngineT: Deref,
{
    type Target = InnerEngineT::Target;
    fn deref(&self) -> &Self::Target {
        self.inner_engine.deref()
    }
}

impl<InnerEngineT> EngineTrait for CliqueConstraintEngine<InnerEngineT>
where
    InnerEngineT: EngineTrait,
    InnerEngineT::CompositeExplainKey:
        From<CliqueConstraintExplainKey> + TryInto<CliqueConstraintExplainKey>,
{
    type CompositeExplainKey = InnerEngineT::CompositeExplainKey;
    type ExplainKey = Either<CliqueConstraintExplainKey, InnerEngineT::ExplainKey>;
    type ExplanationConstraint<'a>
        = Either<impl CliqueConstraintTrait + 'a, InnerEngineT::ExplanationConstraint<'a>>
    where
        Self: 'a;

    fn state(&self) -> State<Self::ExplainKey> {
        return self.state.composite(self.inner_engine.state());
    }

    fn explain(&self, explain_key: Self::ExplainKey) -> Self::ExplanationConstraint<'_> {
        match explain_key {
            Either::Left(explain_key) => {
                return Either::Left(&self.rows[explain_key.row_id]);
            }
            Either::Right(explain_key) => {
                return Either::Right(self.inner_engine.explain(explain_key));
            }
        }
    }

    fn add_variable(&mut self) {
        self.inner_engine.add_variable();
        self.columns.push([Column::default(), Column::default()]);
    }

    fn assign(&mut self, literal: Literal, reason: Reason<Self::CompositeExplainKey>) {
        assert!(self.state.is_noconflict());
        assert!(self.inner_engine.state().is_noconflict());
        debug_assert!(
            self.number_of_confirmed_assignments == self.inner_engine.number_of_assignments()
        );
        if let Reason::Propagation { explain_key } = reason {
            assert!(explain_key.try_into().is_err());
        }

        self.inner_engine.assign(literal, reason);
        self.propagate();
    }

    fn backjump(&mut self, backjump_level: usize) {
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
}

impl<CliqueConstraintT, InnerEngineT, InnerConstraintT>
    EngineAddConstraintTrait<Either<&CliqueConstraintT, InnerConstraintT>>
    for CliqueConstraintEngine<InnerEngineT>
where
    CliqueConstraintT: CliqueConstraintTrait,
    InnerEngineT: EngineTrait + EngineAddConstraintTrait<InnerConstraintT>,
    InnerEngineT::CompositeExplainKey:
        From<CliqueConstraintExplainKey> + TryInto<CliqueConstraintExplainKey>,
{
    fn add_constraint(
        &mut self,
        constraint: Either<&CliqueConstraintT, InnerConstraintT>,
        is_learnt: bool,
    ) {
        match constraint {
            Either::Left(constraint) => {
                // CliqueConstraint を構築
                let constraint: CliqueConstraint = constraint.into();

                // False が割り当てられているリテラルが存在すればその最小の決定レベルを取得
                let min_falsified_decision_level = constraint
                    .iter_literals()
                    .filter(|&literal| self.inner_engine.is_false(literal))
                    .map(|literal| self.inner_engine.get_decision_level(literal.index()))
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
            Either::Right(constraint) => {
                self.inner_engine.add_constraint(constraint, is_learnt);
            }
        }

        self.propagate();
    }
}

impl<InnerEngineT> CliqueConstraintEngine<InnerEngineT>
where
    InnerEngineT: EngineTrait,
    InnerEngineT::CompositeExplainKey: From<CliqueConstraintExplainKey>, // + TryInto<CliqueConstraintExplainKey>
{
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
            debug_assert!(row.literals.iter().find(|&&literal| literal == !assignment).is_some());
            let explain_key = CliqueConstraintExplainKey { row_id };
            for &literal in row.literals.iter() {
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
        debug_assert!(!constraint.iter_literals().any(
            |literal| self.inner_engine.is_false(literal)
                && self.inner_engine.get_decision_level(literal.index())
                    < self.inner_engine.decision_level()
        ));

        // 行を追加
        let row_id = self.rows.len();
        let explain_key = CliqueConstraintExplainKey { row_id };
        self.rows.push(Row {
            literals: constraint.literals,
            is_learnt,
        });
        let row = self.rows.last().unwrap();

        // リテラルを含む列に row_id を追加
        for &literal in row.literals.iter() {
            self.columns[literal].row_ids.push(row_id);
        }

        // false が割り当てられているリテラルの数を算出
        let number_of_falsified_literals =
            row.literals.iter().filter(|&&literal| self.inner_engine.is_false(literal)).count();

        if number_of_falsified_literals == 1 {
            // False が割り当てられているリテラルがちょうど 1 つであれば伝播
            let &falsified_literal =
                row.literals.iter().find(|&&literal| self.inner_engine.is_false(literal)).unwrap();
            for &literal in row.literals.iter() {
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
