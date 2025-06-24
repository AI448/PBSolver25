use super::etc::{Reason, State};
use crate::{
    Literal,
    collections::LiteralArray,
    constraint::{ConstraintView, LinearConstraintTrait, UnsignedIntegerTrait},
    pb_engine::{
        CardinalConstraintExplainKey, CardinalEngine, DecisionStack, OneSatEngineExplainKey,
        cardinal_engine::CardinalEngineExplainKey, two_sat_engine::CliqueConstraintExplainKey,
    },
};
use either::Either;
use std::{
    cmp::{Reverse, max},
    collections::VecDeque,
    fmt::Debug,
    ops::Deref,
};
use utility::Set;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LinearConstraintExplainKey {
    row_id: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LinearEngineExplainKey {
    LinearConstraint(LinearConstraintExplainKey),
    CardinalEngine(CardinalEngineExplainKey),
}

impl From<LinearConstraintExplainKey> for LinearEngineExplainKey {
    fn from(explain_key: LinearConstraintExplainKey) -> Self {
        Self::LinearConstraint(explain_key)
    }
}

impl<ExplainKeyT> From<ExplainKeyT> for LinearEngineExplainKey
where
    ExplainKeyT: Into<CardinalEngineExplainKey>,
{
    fn from(explain_key: ExplainKeyT) -> Self {
        Self::CardinalEngine(explain_key.into())
    }
}

#[derive(Clone, Debug)]
struct Constraint<ValueT> {
    terms: Vec<(Literal, ValueT)>,
    lower: ValueT,
}

#[derive(Clone, Debug)]
struct Row<ValueT> {
    constraint: Constraint<ValueT>,
    is_learnt: bool,
    sup: ValueT,
    max_unassigned_coefficient: ValueT,
}

#[derive(Clone, Debug)]
struct Column<ValueT> {
    terms: Vec<(usize, ValueT)>,
}

impl<ValueT> Default for Column<ValueT> {
    fn default() -> Self {
        Self {
            terms: Vec::default(),
        }
    }
}

pub struct LinearConstraintEngine<ValueT, CompositeExplainKeyT> {
    // strengthen: StrengthenLinearConstraint<u64>,
    state: State<LinearConstraintExplainKey>,
    inner_engine: CardinalEngine<CompositeExplainKeyT>,
    rows: Vec<Row<ValueT>>,
    columns: LiteralArray<Column<ValueT>>,
    number_of_confirmed_assignments: usize,
    constraint_queue: VecDeque<(Constraint<ValueT>, bool)>,
    conflict_rows: Set,
    confirming_rows: Set,
}

impl<ValueT, CompositeExplainKeyT> LinearConstraintEngine<ValueT, CompositeExplainKeyT> {
    pub fn new() -> Self {
        Self {
            // strengthen: StrengthenLinearConstraint::default(),
            inner_engine: CardinalEngine::new(),
            state: State::Noconflict,
            rows: Vec::default(),
            columns: LiteralArray::default(),
            number_of_confirmed_assignments: 0,
            constraint_queue: VecDeque::default(),
            conflict_rows: Set::default(),
            confirming_rows: Set::default(),
        }
    }
}

impl<ValueT, CompositeExplainKeyT> Deref for LinearConstraintEngine<ValueT, CompositeExplainKeyT> {
    type Target = DecisionStack<CompositeExplainKeyT>;
    fn deref(&self) -> &Self::Target {
        self.inner_engine.deref()
    }
}

impl<ValueT, CompositeExplainKeyT> LinearConstraintEngine<ValueT, CompositeExplainKeyT>
where
    ValueT: UnsignedIntegerTrait,
    CompositeExplainKeyT: From<LinearConstraintExplainKey>
        + From<CardinalConstraintExplainKey>
        + From<CliqueConstraintExplainKey>
        + From<OneSatEngineExplainKey>,
{
    pub fn state(&self) -> State<LinearEngineExplainKey> {
        return self.state.composite(self.inner_engine.state());
    }

    pub fn explain(
        &self,
        explain_key: LinearEngineExplainKey,
    ) -> impl LinearConstraintTrait<Value = ValueT> {
        return match explain_key {
            LinearEngineExplainKey::LinearConstraint(explain_key) => {
                let row = &self.rows[explain_key.row_id];
                Either::Left(ConstraintView::new(
                    row.constraint.terms.iter().cloned(),
                    row.constraint.lower,
                ))
            }
            LinearEngineExplainKey::CardinalEngine(explain_key) => {
                Either::Right(self.inner_engine.explain(explain_key))
            }
        };
    }

    pub fn add_variable(&mut self) {
        self.inner_engine.add_variable();
        self.columns.push([Column::default(), Column::default()]);
    }

    pub fn assign(&mut self, literal: Literal, reason: Reason<CompositeExplainKeyT>) {
        self.inner_engine.assign(literal, reason);
        self.propagate();
    }

    pub fn backjump(&mut self, backjump_level: usize) {
        let backjump_order = self.inner_engine.order_range(backjump_level).end;
        debug_assert!(backjump_order <= self.number_of_confirmed_assignments);
        while self.number_of_confirmed_assignments > backjump_order {
            self.number_of_confirmed_assignments -= 1;
            let unassignment =
                self.inner_engine.get_assignment(self.number_of_confirmed_assignments);
            for &(row_id, coefficient) in self.columns[!unassignment].terms.iter() {
                let row = &mut self.rows[row_id];
                row.sup = row.sup + coefficient;
                row.max_unassigned_coefficient = max(row.max_unassigned_coefficient, coefficient);
            }
            for &(row_id, coefficient) in self.columns[unassignment].terms.iter() {
                let row = &mut self.rows[row_id];
                row.max_unassigned_coefficient = max(row.max_unassigned_coefficient, coefficient);
            }
        }

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

    pub fn add_constraint(
        &mut self,
        linear_constraint: &impl LinearConstraintTrait<Value = ValueT>,
        is_learnt: bool,
    ) {
        if linear_constraint.lower() == ValueT::zero() {
            return;
        }

        if linear_constraint.iter_terms().all(|(_, coefficient)| coefficient == ValueT::one()) {
            self.inner_engine.add_constraint(linear_constraint, is_learnt);
        } else {
            // そうでなければ伝播の発生を確認して状態を更新し，制約をキューに追加

            let mut constraint = Constraint {
                terms: Vec::from_iter(linear_constraint.iter_terms()),
                lower: linear_constraint.lower(),
            };

            // 伝播が発生する最小の決定レベルを特定
            // TODO: 無駄に浅い階層までバックジャンプすることがあるので，CalculatePropagationLevel を使う
            let min_propagation_level = {
                // 現在の上界と未割り当て変数の係数の最大値を算出
                let mut sup = ValueT::zero();
                let mut max_unassigned_coefficient = ValueT::zero();
                for (literal, coefficient) in constraint.terms.iter().cloned() {
                    if !self.inner_engine.is_false(literal) {
                        sup = sup + coefficient;
                    }
                    if !self.inner_engine.is_assigned(literal.index()) {
                        max_unassigned_coefficient = max(max_unassigned_coefficient, coefficient);
                    }
                }
                // 割り当て順にソート
                constraint.terms.sort_unstable_by_key(|&(literal, _)| {
                    self.inner_engine.get_assignment_order(literal.index())
                });
                // 割り当ての逆順に走査
                let mut min_propagation_level = None;
                for (literal, coefficient) in constraint.terms.iter().rev().cloned() {
                    if sup - max_unassigned_coefficient < constraint.lower {
                        min_propagation_level =
                            Some(self.inner_engine.get_decision_level(literal.index()));
                    }
                    if self.inner_engine.is_false(literal) {
                        sup = sup + coefficient;
                    }
                    if self.inner_engine.is_assigned(literal.index()) {
                        max_unassigned_coefficient = max(max_unassigned_coefficient, coefficient);
                    }
                }
                if sup - max_unassigned_coefficient < constraint.lower {
                    min_propagation_level = Some(0);
                }
                min_propagation_level
            };

            // 現在の決定レベルよりも前に伝播が発生するなら state を BackjumpRequired に
            if let Some(min_propagation_level) = min_propagation_level
                && min_propagation_level < self.inner_engine.decision_level()
            {
                self.state.merge(State::BackjumpRequired {
                    backjump_level: min_propagation_level,
                });
                debug_assert!(self.state.is_backjump_required());
            }

            // 制約条件をキューに追加
            self.constraint_queue.push_back((constraint, is_learnt));
        }

        self.propagate();
    }

    fn propagate(&mut self) {
        self.conflict_rows.clear();
        self.confirming_rows.clear();

        while self.state.is_noconflict() && self.inner_engine.state().is_noconflict() {
            if !self.conflict_rows.is_empty() {
                // 矛盾している制約が現れた場合には state を Conflict に
                let &row_id = self.conflict_rows.iter().next().unwrap();
                let row = &self.rows[row_id];
                eprintln!(
                    "CONFLICT {} {} {}",
                    row.sup, row.constraint.lower, row.max_unassigned_coefficient
                );
                self.state = State::Conflict {
                    explain_key: LinearConstraintExplainKey { row_id },
                };
                break;
            } else if self.number_of_confirmed_assignments
                < self.inner_engine.number_of_assignments()
            {
                // 未確認の割り当てがある場合には割り当てを確認
                self.confirm_assignment();
            } else if !self.confirming_rows.is_empty() {
                // 伝播を引き起こす可能性がある制約条件が現れた場合には制約条件を確認
                self.confirm_row();
            } else if !self.constraint_queue.is_empty() {
                // 追加すべき行が存在する場合には行を追加
                self.add_row();
            } else {
                break;
            }
        }
    }

    /// 割り当てを確認し，行の状態を更新する
    /// * number_of_confirmed_assignments がインクリメントされる
    /// * sup が更新される
    /// * conflict_rows, confirming_rows に要素が追加されることがある
    /// * inner_engine は変更されない
    fn confirm_assignment(&mut self) {
        debug_assert!(self.inner_engine.state().is_noconflict());
        debug_assert!(self.state.is_noconflict());
        debug_assert!(
            self.number_of_confirmed_assignments < self.inner_engine.number_of_assignments()
        );

        // 上界を更新
        let assignment = self.inner_engine.get_assignment(self.number_of_confirmed_assignments);
        self.number_of_confirmed_assignments += 1;
        for &(row_id, coefficient) in self.columns[!assignment].terms.iter() {
            let row = &mut self.rows[row_id];
            row.sup = row.sup - coefficient;
            if row.sup < row.constraint.lower {
                self.conflict_rows.insert(row_id);
            } else if row.sup < row.constraint.lower + row.max_unassigned_coefficient {
                self.confirming_rows.insert(row_id);
            }
        }
    }

    /// 行による伝播を確認する
    /// * confirming_rows から要素が一つ取り出される
    /// * max_unassigned_coefficient が更新される
    /// * inner_engine が更新される
    fn confirm_row(&mut self) {
        debug_assert!(self.inner_engine.state().is_noconflict());
        debug_assert!(self.state.is_noconflict());
        debug_assert!(
            self.number_of_confirmed_assignments == self.inner_engine.number_of_assignments()
        );
        let row_id = self.confirming_rows.pop().unwrap();

        let mut k = 0;

        // max_unassigned_coefficient を更新
        // NOTE: terms が係数の降順にソートされていることを前提としている
        let row = &mut self.rows[row_id];
        debug_assert!(row.sup >= row.constraint.lower);
        debug_assert!(row.sup < row.constraint.lower + row.max_unassigned_coefficient);
        row.max_unassigned_coefficient = ValueT::zero();
        while k < row.constraint.terms.len() {
            let (literal, coefficient) = row.constraint.terms[k];
            if !self.inner_engine.is_assigned(literal.index()) {
                row.max_unassigned_coefficient = coefficient;
                break;
            }
            k += 1;
        }
        // NOTE: これ以降では number_of_evaluated_assignments と inner_engine.number_of_assigneds がずれるため
        // row を変更してはいけない
        let row = &self.rows[row_id];

        // 伝播が発生する場合
        if row.max_unassigned_coefficient > row.sup - row.constraint.lower {
            let explain_key = LinearConstraintExplainKey { row_id };
            while k < row.constraint.terms.len() {
                let (literal, coefficient) = row.constraint.terms[k];
                if row.sup - coefficient < row.constraint.lower {
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
                    } else if self.inner_engine.is_false(literal)
                        && self.inner_engine.get_assignment_order(literal.index())
                            >= self.number_of_confirmed_assignments
                    {
                        eprintln!("CONFLICT");
                        self.state = State::Conflict { explain_key };
                        return;
                    }
                } else {
                    break;
                }
                k += 1;
            }
        }
    }

    /// 行を追加する
    /// * constraint_queue から要素が一つ取り出されて rows に行が追加され， columns に項が追加される
    /// * conflict_rows, confirming_rows に要素が追加されることがある
    /// * inner_engine は更新されない
    fn add_row(&mut self) {
        debug_assert!(self.state.is_noconflict());
        debug_assert!(self.inner_engine.state().is_noconflict());
        debug_assert!(!self.constraint_queue.is_empty());
        // NOTE: < でも動作するような実装は可能だが面倒なので == を仮定する
        debug_assert!(
            self.number_of_confirmed_assignments == self.inner_engine.number_of_assignments()
        );

        // キューから制約を取り出し
        let (constraint, is_learnt) = self.constraint_queue.pop_front().unwrap();

        // 現在の決定レベルが 0 でない場合，現在の決定レベルよりも前に伝播が発生することはないはず
        #[cfg(debug_assertions)]
        if self.decision_level() != 0 {
            let mut sup_at_previous_decision_level = ValueT::zero();
            for &(literal, coefficient) in constraint.terms.iter() {
                if !(self.inner_engine.is_false(literal)
                    && self.inner_engine.get_decision_level(literal.index())
                        < self.inner_engine.decision_level())
                {
                    sup_at_previous_decision_level = sup_at_previous_decision_level + coefficient;
                }
            }
            debug_assert!(sup_at_previous_decision_level >= constraint.lower);
        }

        // 上界と未割り当てリテラルの係数の最大値を計算
        let mut sup = ValueT::zero();
        let mut max_unassigned_coefficient = ValueT::zero();
        for &(literal, coefficient) in constraint.terms.iter() {
            if !self.inner_engine.is_false(literal) {
                sup = sup + coefficient;
            }
            if !self.inner_engine.is_assigned(literal.index()) {
                max_unassigned_coefficient = max(max_unassigned_coefficient, coefficient);
            }
        }

        // 行を追加
        let row_id = self.rows.len();
        self.rows.push(Row {
            constraint,
            is_learnt,
            sup,
            max_unassigned_coefficient,
        });
        let row = self.rows.last_mut().unwrap();

        // 係数の降順にソート
        row.constraint.terms.sort_unstable_by_key(|&(_, coefficient)| Reverse(coefficient));

        // 列に項を追加
        for &(literal, coefficient) in row.constraint.terms.iter() {
            self.columns[literal].terms.push((row_id, coefficient));
        }

        if row.sup < row.constraint.lower {
            // 違反している場合
            self.conflict_rows.insert(row_id);
        // 伝播が発生する場合
        } else if row.sup - row.max_unassigned_coefficient < row.constraint.lower {
            self.confirming_rows.insert(row_id);
        }
    }
}
