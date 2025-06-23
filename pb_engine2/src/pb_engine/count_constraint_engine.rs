use std::{collections::VecDeque, ops::Deref};

use either::Either;
use num::{FromPrimitive, One, ToPrimitive, Zero};

use crate::{
    Literal,
    collections::LiteralArray,
    constraint::{ConstraintView, LinearConstraintTrait, UnsignedIntegerTrait},
    pb_engine::{OneSatEngine, OneSatEngineExplainKey},
};

use super::etc::{Reason, State};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CardinalConstraintExplainKey {
    row_id: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CardinalEngineExplainKey {
    CardinalConstraint(CardinalConstraintExplainKey),
    OneSatEngine(OneSatEngineExplainKey),
}

impl From<CardinalConstraintExplainKey> for CardinalEngineExplainKey {
    fn from(explain_key: CardinalConstraintExplainKey) -> Self {
        Self::CardinalConstraint(explain_key)
    }
}

impl<ExplainKeyT> From<ExplainKeyT> for CardinalEngineExplainKey
where
    ExplainKeyT: Into<OneSatEngineExplainKey>,
{
    fn from(explain_key: ExplainKeyT) -> Self {
        Self::OneSatEngine(explain_key.into())
    }
}

#[derive(Clone, Debug)]
struct CardinalConstraint {
    literals: Vec<Literal>,
    lower: usize,
}

#[derive(Clone, Debug)]
struct Row {
    constraint: CardinalConstraint,
    is_learnt: bool,
}

impl Row {
    fn number_of_watched_literals(&self) -> usize {
        return self.constraint.lower + 1;
    }
}

#[derive(Clone, Copy, Debug)]
struct Watcher {
    row_id: usize,
    position: usize,
}

#[derive(Default, Clone, Debug)]
struct Column {
    watchers: Vec<Watcher>,
}

pub struct CardinalEngine<CompositeExplainKey> {
    state: State<CardinalConstraintExplainKey>,
    inner_engine: OneSatEngine<CompositeExplainKey>,
    rows: Vec<Row>,
    columns: LiteralArray<Column>,
    number_of_confirmed_assignments: usize,
    constraint_queue: VecDeque<(CardinalConstraint, bool)>,
}

impl<CompositeExplainKeyT> CardinalEngine<CompositeExplainKeyT> {
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

impl<CompositeExplainKeyT> Deref for CardinalEngine<CompositeExplainKeyT> {
    type Target = <OneSatEngine<CompositeExplainKeyT> as Deref>::Target;
    fn deref(&self) -> &Self::Target {
        self.inner_engine.deref()
    }
}

impl<CompositeExplainKeyT> CardinalEngine<CompositeExplainKeyT>
where
    CompositeExplainKeyT: From<CardinalConstraintExplainKey> + From<OneSatEngineExplainKey>,
{
    pub fn state(&self) -> State<CardinalEngineExplainKey> {
        return self.state.composite(self.inner_engine.state());
    }

    pub fn explain<ValueT>(
        &self,
        explain_key: CardinalEngineExplainKey,
    ) -> impl LinearConstraintTrait<Value = ValueT>
    where
        ValueT: UnsignedIntegerTrait,
    {
        return match explain_key {
            CardinalEngineExplainKey::CardinalConstraint(explain_key) => {
                let constraint = &self.rows[explain_key.row_id].constraint;
                Either::Left(ConstraintView::new(
                    constraint.literals.iter().map(|&literal| (literal, ValueT::one())),
                    ValueT::from_usize(constraint.lower).unwrap(),
                ))
            }
            CardinalEngineExplainKey::OneSatEngine(explain_key) => {
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
        if self.state.is_backjump_required_and(|required_backjump_level| {
            backjump_level <= required_backjump_level
        }) || self.state.is_conflict()
        {
            self.state = State::Noconflict;
        }
        self.propagate();
    }

    pub fn add_constraint<ConstraintT>(&mut self, linear_constraint: &ConstraintT, is_learnt: bool)
    where
        ConstraintT: LinearConstraintTrait,
    {
        assert!(
            linear_constraint
                .iter_terms()
                .all(|(_, coefficient)| coefficient == ConstraintT::Value::one())
        );

        if linear_constraint.lower() == ConstraintT::Value::zero() {
            return;
        }

        // 決定レベル 0 での左辺値の上界
        let sup0 = ConstraintT::Value::from_usize(linear_constraint.iter_terms().count()).unwrap();

        if sup0 <= linear_constraint.lower() {
            // 左辺値の上界が右辺値以下であれば inner_engine に追加
            self.inner_engine.add_constraint(linear_constraint, is_learnt);
        } else {
            let mut constraint = CardinalConstraint {
                literals: Vec::from_iter(
                    linear_constraint.iter_terms().map(|(literal, _)| literal),
                ),
                lower: linear_constraint.lower().to_usize().unwrap(),
            };

            // 伝播が発生する最小の決定レベルを特定
            let backjump_level = if constraint.literals.len() > constraint.lower {
                constraint.literals.sort_unstable_by_key(|literal| {
                    self.inner_engine.get_assignment_order(literal.index())
                });
                let nth_falsified_literal = constraint
                    .literals
                    .iter()
                    .cloned()
                    .filter(|&literal| self.inner_engine.is_false(literal))
                    .nth(constraint.literals.len() - constraint.lower - 1);
                nth_falsified_literal
                    .map(|literal| self.inner_engine.get_decision_level(literal.index()))
            } else {
                Some(0)
            };
            if is_learnt {
                let sup_at_previous_decision_level = constraint
                    .literals
                    .iter()
                    .cloned()
                    .filter(|&literal| {
                        !(self.inner_engine.is_false(literal)
                            && self.get_decision_level(literal.index())
                                < self.inner_engine.decision_level())
                    })
                    .count();
                eprintln!(
                    "{:?} {}, {}, {}",
                    backjump_level,
                    self.inner_engine.decision_level(),
                    sup_at_previous_decision_level,
                    constraint.lower
                );
            }
            // 現在の決定レベルよりも前に伝播が発生するなら state を BackjumpRequired に
            if let Some(backjump_level) = backjump_level
                && backjump_level < self.inner_engine.decision_level()
            {
                self.state.merge(State::BackjumpRequired { backjump_level });
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

        'for_watcher: for w in (0..self.columns[!assignment].watchers.len()).rev() {
            let watcher = self.columns[!assignment].watchers[w];
            let row = &mut self.rows[watcher.row_id];
            debug_assert!(watcher.position < row.number_of_watched_literals());
            debug_assert!(row.constraint.literals[watcher.position] == !assignment);

            for p in row.number_of_watched_literals()..row.constraint.literals.len() {
                let literal = row.constraint.literals[p];
                if !self.inner_engine.is_false(literal) {
                    row.constraint.literals.swap(watcher.position, p);
                    self.columns[!assignment].watchers.swap_remove(w);
                    self.columns[literal].watchers.push(watcher);
                    continue 'for_watcher;
                }
            }

            let explain_key = CardinalConstraintExplainKey {
                row_id: watcher.row_id,
            };

            for &literal in row.constraint.literals[..row.number_of_watched_literals()].iter() {
                if literal == !assignment {
                    continue;
                }
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
                } else if self.inner_engine.is_false(literal) {
                    // eprintln!("CONFLICT COUNT_CONSTRAINT {}", line!());
                    self.state.merge(State::Conflict { explain_key });
                    return;
                }
            }
        }
    }

    fn propagate_by_constraint_addition(&mut self) {
        debug_assert!(self.state.is_noconflict());
        debug_assert!(self.inner_engine.state().is_noconflict());
        // NOTE: < の状態でこの処理を行うのは面倒なので == を仮定した実装とする
        debug_assert!(
            self.number_of_confirmed_assignments == self.inner_engine.number_of_assignments()
        );
        debug_assert!(!self.constraint_queue.is_empty());

        // キューから制約を取り出し
        let (constraint, is_learnt) = self.constraint_queue.pop_front().unwrap();
        debug_assert!(constraint.lower >= 1);
        // 現在の決定レベルが 0 でない場合，現在の決定レベルよりも前に伝播が発生することはないはず
        #[cfg(debug_assertions)]
        if self.inner_engine.decision_level() != 0 {
            let sup_at_prev_decision_level = constraint
                .literals
                .iter()
                .cloned()
                .filter(|&literal| {
                    !(self.inner_engine.is_false(literal)
                        && self.inner_engine.get_decision_level(literal.index())
                            < self.inner_engine.decision_level())
                })
                .count();
            debug_assert!(sup_at_prev_decision_level >= constraint.lower);
        }

        // 行を追加
        let row_id = self.rows.len();
        let explain_key = CardinalConstraintExplainKey { row_id };
        self.rows.push(Row {
            constraint: constraint,
            is_learnt: is_learnt,
        });
        let row = self.rows.last_mut().unwrap();

        if row.constraint.literals.len() < row.constraint.lower {
            // 充足不可能な制約条件である場合
            debug_assert!(self.inner_engine.decision_level() == 0);
            // NOTE: 監視リテラルは不要
            eprintln!("CONFLICT COUNT_CONSTRAINT {}", line!());
            self.state = State::Conflict { explain_key };
            return;
        } else if row.constraint.literals.len() == row.constraint.lower {
            // すべてのリテラルが固定される場合
            debug_assert!(self.inner_engine.decision_level() == 0);
            // NOTE: 監視リテラルは不要
            // すべてのリテラルに True を割り当て
            for &literal in row.constraint.literals.iter() {
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
                } else if self.inner_engine.is_false(literal) {
                    eprintln!("CONFLICT COUNT_CONSTRAINT {}", line!());
                    self.state = State::Conflict { explain_key };
                    return;
                }
            }
        } else {
            // それ以外
            // True が割り当たっているものを割り当ての昇順・未割り当て・Fase が割り当たっているものを割り当ての降順にソート
            row.constraint.literals.sort_unstable_by_key(|&literal| {
                if self.inner_engine.is_true(literal) {
                    (0, self.inner_engine.get_assignment_order(literal.index()))
                } else if !self.inner_engine.is_assigned(literal.index()) {
                    (1, 0)
                } else {
                    (
                        2,
                        usize::MAX - self.inner_engine.get_assignment_order(literal.index()),
                    )
                }
            });
            // 監視を追加
            for (position, &literal) in
                row.constraint.literals[..row.number_of_watched_literals()].iter().enumerate()
            {
                self.columns[literal].watchers.push(Watcher { row_id, position });
            }

            // 矛盾している場合
            if self
                .inner_engine
                .is_false(row.constraint.literals[row.number_of_watched_literals() - 2])
            {
                // state を Conflict に
                eprintln!("CONFLICT COUNT_CONSTRAINT");
                self.state = State::Conflict { explain_key };
                return;

            // 伝播が発生する場合
            } else if self
                .inner_engine
                .is_false(row.constraint.literals[row.number_of_watched_literals() - 1])
            {
                // 末尾以外の監視リテラルに True を割り当て
                for &literal in
                    row.constraint.literals[..row.number_of_watched_literals() - 1].iter()
                {
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
                    } else if self.inner_engine.is_false(literal) {
                        eprintln!("CONFLICT COUNT_CONSTRAINT {}", line!());
                        self.state = State::Conflict { explain_key };
                        return;
                    }
                }
            }
        }
    }
}
