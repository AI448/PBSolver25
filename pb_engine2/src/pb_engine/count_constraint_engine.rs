use std::{collections::VecDeque, ops::Deref};

use either::Either;

use crate::{
    Literal,
    collections::LiteralArray,
    pb_engine::{
        DecisionStack, MonadicConstraint, MonadicConstraintEngine, MonadicConstraintExplainKey,
        decision_stack, monadic_constraint_engine::OneSatExplainKey,
    },
};

use super::etc::{Reason, State};

pub trait CountConstraintTrait {
    fn iter_terms(&self) -> impl Iterator<Item = Literal> + Clone;
    fn lower(&self) -> usize;
    fn len(&self) -> usize {
        self.iter_terms().count()
    }

    fn drop_fixed_variables<ExplainKeyT>(
        &self,
        decision_stack: &DecisionStack<ExplainKeyT>,
    ) -> impl CountConstraintTrait {
        let mut lower = self.lower();
        for literal in self.iter_terms() {
            if lower == 0 {
                break;
            }
            if decision_stack.get_decision_level(literal.index()) == 0
                && decision_stack.is_true(literal)
            {
                lower -= 1;
            }
        }
        return CountConstraintView::new(
            self.iter_terms()
                .filter(|&literal| decision_stack.get_decision_level(literal.index()) != 0),
            lower,
        );
    }
}

impl<LeftCountConstraintT, RightConstraintT> CountConstraintTrait
    for Either<LeftCountConstraintT, RightConstraintT>
where
    LeftCountConstraintT: CountConstraintTrait,
    RightConstraintT: CountConstraintTrait,
{
    fn iter_terms(&self) -> impl Iterator<Item = Literal> + Clone {
        return match self {
            Either::Left(left) => Either::Left(left.iter_terms()),
            Either::Right(right) => Either::Right(right.iter_terms()),
        };
    }

    fn lower(&self) -> usize {
        return match self {
            Either::Left(left) => left.lower(),
            Either::Right(right) => right.lower(),
        };
    }
}

#[derive(Clone, Debug)]
pub struct CountConstraint {
    literals: Vec<Literal>,
    lower: usize,
}

impl<CountConstraintT> From<&CountConstraintT> for CountConstraint
where
    CountConstraintT: CountConstraintTrait,
{
    fn from(count_constraint: &CountConstraintT) -> Self {
        return Self {
            literals: count_constraint.iter_terms().collect(),
            lower: count_constraint.lower(),
        };
    }
}

impl CountConstraintTrait for CountConstraint {
    fn iter_terms(&self) -> impl Iterator<Item = Literal> + Clone {
        self.literals.iter().cloned()
    }

    fn lower(&self) -> usize {
        self.lower
    }
}

#[derive(Clone, Debug)]
pub struct CountConstraintView<IteratorT> {
    literals: IteratorT,
    lower: usize,
}

impl<IteratorT> CountConstraintView<IteratorT> {
    pub fn new(literals: IteratorT, lower: usize) -> Self {
        Self { literals, lower }
    }
}

impl<IteratorT> CountConstraintTrait for CountConstraintView<IteratorT>
where
    IteratorT: Iterator<Item = Literal> + Clone,
{
    fn iter_terms(&self) -> impl Iterator<Item = Literal> + Clone {
        self.literals.clone()
    }

    fn lower(&self) -> usize {
        self.lower
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CountConstraintExplainKey {
    row_id: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CountConstraintEngineExplainKey {
    OneSatEngine(OneSatExplainKey),
    CountConstraint(CountConstraintExplainKey),
}

impl From<OneSatExplainKey> for CountConstraintEngineExplainKey {
    fn from(explain_key: OneSatExplainKey) -> Self {
        Self::OneSatEngine(explain_key)
    }
}

impl From<CountConstraintExplainKey> for CountConstraintEngineExplainKey {
    fn from(explain_key: CountConstraintExplainKey) -> Self {
        Self::CountConstraint(explain_key)
    }
}

#[derive(Clone, Debug)]
struct Row {
    literals: Vec<Literal>,
    lower: usize,
    is_learnt: bool,
}

impl Row {
    fn number_of_watched_literals(&self) -> usize {
        return self.lower + 1;
    }
}

impl CountConstraintTrait for &Row {
    fn iter_terms(&self) -> impl Iterator<Item = Literal> + Clone {
        self.literals.iter().cloned()
    }
    fn lower(&self) -> usize {
        self.lower
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

pub struct CountConstraintEngine<CompositeExplainKey> {
    state: State<CountConstraintExplainKey>,
    inner_engine: MonadicConstraintEngine<CompositeExplainKey>,
    rows: Vec<Row>,
    columns: LiteralArray<Column>,
    number_of_confirmed_assignments: usize,
    constraint_queue: VecDeque<(CountConstraint, bool)>,
}

impl<CompositeExplainKeyT> CountConstraintEngine<CompositeExplainKeyT> {
    pub fn new() -> Self {
        Self {
            state: State::Noconflict,
            inner_engine: MonadicConstraintEngine::new(),
            rows: Vec::default(),
            columns: LiteralArray::default(),
            number_of_confirmed_assignments: 0,
            constraint_queue: VecDeque::default(),
        }
    }
}

impl<CompositeExplainKeyT> Deref for CountConstraintEngine<CompositeExplainKeyT> {
    type Target = <MonadicConstraintEngine<CompositeExplainKeyT> as Deref>::Target;
    fn deref(&self) -> &Self::Target {
        self.inner_engine.deref()
    }
}

impl<CompositeExplainKeyT> CountConstraintEngine<CompositeExplainKeyT>
where
    CompositeExplainKeyT: From<CountConstraintExplainKey> + From<MonadicConstraintExplainKey>,
{
    pub fn state(&self) -> State<CountConstraintEngineExplainKey> {
        return self.state.composite(self.inner_engine.state());
    }

    pub fn explain(
        &self,
        explain_key: CountConstraintEngineExplainKey,
    ) -> impl CountConstraintTrait {
        return match explain_key {
            CountConstraintEngineExplainKey::OneSatEngine(explain_key) => {
                Either::Left(CountConstraintView::new(
                    [self.inner_engine.explain(explain_key).literal].into_iter(),
                    1,
                ))
            }
            CountConstraintEngineExplainKey::CountConstraint(explain_key) => {
                Either::Right(&self.rows[explain_key.row_id])
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

    pub fn add_constraint(&mut self, constraint: &impl CountConstraintTrait, is_learnt: bool) {
        let mut constraint: CountConstraint =
            (&constraint.drop_fixed_variables(&self.inner_engine)).into();
        if constraint.lower() == 0 {
            // 無効な制約条件であれば何もしない
            return;
        } else if constraint.len() == 1 {
            // 項が 1 つであれば inner_engine に追加する
            self.inner_engine.add_constraint(
                MonadicConstraint {
                    literal: constraint.iter_terms().next().unwrap(),
                },
                is_learnt,
            );
        } else {
            // 伝播が発生する最小の決定レベルを特定
            let backjump_level = if constraint.len() > constraint.lower {
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
            debug_assert!(row.literals[watcher.position] == !assignment);

            for p in row.number_of_watched_literals()..row.literals.len() {
                let literal = row.literals[p];
                if !self.inner_engine.is_false(literal) {
                    row.literals.swap(watcher.position, p);
                    self.columns[!assignment].watchers.swap_remove(w);
                    self.columns[literal].watchers.push(watcher);
                    continue 'for_watcher;
                }
            }

            let explain_key = CountConstraintExplainKey {
                row_id: watcher.row_id,
            };

            for &literal in row.literals[..row.number_of_watched_literals()].iter() {
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
                    eprintln!("CONFLICT COUNT_CONSTRAINT {}", line!());
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
        let explain_key = CountConstraintExplainKey { row_id };
        self.rows.push(Row {
            literals: constraint.literals,
            lower: constraint.lower,
            is_learnt: is_learnt,
        });
        let row = self.rows.last_mut().unwrap();

        if row.literals.len() < row.lower {
            // 充足不可能な制約条件である場合
            debug_assert!(self.inner_engine.decision_level() == 0);
            // NOTE: 監視リテラルは不要
            eprintln!("CONFLICT COUNT_CONSTRAINT {}", line!());
            self.state = State::Conflict { explain_key };
            return;
        } else if row.literals.len() == row.lower {
            // すべてのリテラルが固定される場合
            debug_assert!(self.inner_engine.decision_level() == 0);
            // NOTE: 監視リテラルは不要
            // すべてのリテラルに True を割り当て
            for &literal in row.literals.iter() {
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
            row.literals.sort_unstable_by_key(|&literal| {
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
                row.literals[..row.number_of_watched_literals()].iter().enumerate()
            {
                self.columns[literal].watchers.push(Watcher { row_id, position });
            }

            // 矛盾している場合
            if self.inner_engine.is_false(row.literals[row.number_of_watched_literals() - 2]) {
                // state を Conflict に
                eprintln!("CONFLICT COUNT_CONSTRAINT");
                self.state = State::Conflict { explain_key };
                return;

            // 伝播が発生する場合
            } else if self.inner_engine.is_false(row.literals[row.number_of_watched_literals() - 1])
            {
                // 末尾以外の監視リテラルに True を割り当て
                for &literal in row.literals[..row.number_of_watched_literals() - 1].iter() {
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
