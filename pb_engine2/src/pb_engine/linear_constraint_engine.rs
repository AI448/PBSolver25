use super::{
    engine_trait::{EngineAddConstraintTrait, EngineTrait},
    etc::{Reason, State},
};
use crate::{Boolean, Literal, collections::LiteralArray};
use either::Either;
use num::{FromPrimitive, Integer, NumCast, ToPrimitive, Unsigned};
use std::{
    cmp::{Reverse, max},
    collections::VecDeque,
    fmt::Debug,
    ops::Deref,
};
use utility::{Map, Set};

pub trait LinearConstraintTrait {
    type Value: Integer + Unsigned + Copy + FromPrimitive;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone;
    fn lower(&self) -> Self::Value;

    fn mul(&self, multipler: Self::Value) -> impl LinearConstraintTrait<Value = Self::Value> {
        return LinearConstraintView::new(
            self.iter_terms().map(move |(literal, coefficient)| (literal, coefficient * multipler)),
            self.lower() * multipler,
        );
    }

    fn convert<ValueT>(&self) -> impl LinearConstraintTrait<Value = ValueT>
    where
        Self::Value: ToPrimitive,
        ValueT: Integer + Unsigned + Copy + FromPrimitive + NumCast,
    {
        return LinearConstraintView::new(
            self.iter_terms()
                .map(|(literal, coefficient)| (literal, ValueT::from(coefficient).unwrap())),
            ValueT::from(self.lower()).unwrap(),
        );
    }
}

impl<LinearConstraintT> LinearConstraintTrait for &LinearConstraintT
where
    LinearConstraintT: LinearConstraintTrait,
{
    type Value = LinearConstraintT::Value;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        (*self).iter_terms()
    }

    fn lower(&self) -> Self::Value {
        (*self).lower()
    }
}

impl<LeftLinearConstraintT, RightLinearConstraintT, ValueT> LinearConstraintTrait
    for Either<LeftLinearConstraintT, RightLinearConstraintT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive,
    LeftLinearConstraintT: LinearConstraintTrait<Value = ValueT>,
    RightLinearConstraintT: LinearConstraintTrait<Value = ValueT>,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        return match self {
            Either::Left(left) => Either::Left(left.iter_terms()),
            Either::Right(right) => Either::Right(right.iter_terms()),
        };
    }
    fn lower(&self) -> Self::Value {
        return match self {
            Either::Left(left) => left.lower(),
            Either::Right(right) => right.lower(),
        };
    }
}

#[derive(Clone, Debug)]
pub struct LinearConstraint<ValueT> {
    terms: Vec<(Literal, ValueT)>,
    lower: ValueT,
}

impl<ValueT> Default for LinearConstraint<ValueT>
where
    ValueT: Integer + Unsigned + Clone + FromPrimitive,
{
    fn default() -> Self {
        Self {
            terms: Vec::default(),
            lower: ValueT::zero(),
        }
    }
}

impl<ValueT> LinearConstraint<ValueT> {
    pub fn replace(&mut self, terms: impl Iterator<Item = (Literal, ValueT)>, lower: ValueT) {
        self.terms.clear();
        self.terms.extend(terms);
        self.lower = lower;
    }

    pub fn replace_by_linear_constraint(
        &mut self,
        linear_constraint: impl LinearConstraintTrait<Value = ValueT>,
    ) {
        self.replace(linear_constraint.iter_terms(), linear_constraint.lower());
    }
}

impl<ValueT, LinearConstraintT> From<&LinearConstraintT> for LinearConstraint<ValueT>
where
    LinearConstraintT: LinearConstraintTrait<Value = ValueT>,
{
    fn from(count_constraint: &LinearConstraintT) -> Self {
        return Self {
            terms: count_constraint.iter_terms().collect(),
            lower: count_constraint.lower(),
        };
    }
}

impl<ValueT> LinearConstraintTrait for LinearConstraint<ValueT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.terms.iter().cloned()
    }

    fn lower(&self) -> Self::Value {
        self.lower
    }
}

#[derive(Clone, Debug)]
pub struct LinearConstraintView<ValueT, IteratorT> {
    terms: IteratorT,
    lower: ValueT,
}

impl<ValueT, IteratorT> LinearConstraintView<ValueT, IteratorT> {
    pub fn new(terms: IteratorT, lower: ValueT) -> Self {
        Self { terms, lower }
    }
}

impl<ValueT, IteratorT> LinearConstraintTrait for LinearConstraintView<ValueT, IteratorT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive,
    IteratorT: Iterator<Item = (Literal, ValueT)> + Clone,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.terms.clone()
    }

    fn lower(&self) -> Self::Value {
        self.lower
    }
}

#[derive(Default, Clone)]
pub struct RandomAccessibleLinearConstraint<ValueT>
where
    ValueT: Copy,
{
    terms: Map<(Boolean, ValueT)>,
    lower: ValueT,
}

impl<ValueT> LinearConstraintTrait for RandomAccessibleLinearConstraint<ValueT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone + '_ {
        return self
            .terms
            .iter()
            .map(|(&index, term)| (Literal::new(index, term.0), term.1.clone()));
    }

    fn lower(&self) -> Self::Value {
        return self.lower;
    }
}

impl<ValueT> RandomAccessibleLinearConstraint<ValueT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive,
{
    pub fn get(&self, literal: Literal) -> Option<ValueT> {
        return self
            .terms
            .get(literal.index())
            .filter(|term| term.0 == literal.value())
            .map(|term| term.1.clone());
    }

    pub fn get_mut(&mut self, literal: Literal) -> Option<&mut ValueT> {
        return self
            .terms
            .get_mut(literal.index())
            .filter(|term| term.0 == literal.value())
            .map(|term| &mut term.1);
    }

    pub fn replace_by_linear_constraint(
        &mut self,
        linear_constraint: impl LinearConstraintTrait<Value = ValueT>,
    ) {
        self.terms.clear();
        self.terms.extend(
            linear_constraint
                .iter_terms()
                .filter(|(_, coefficient)| *coefficient != ValueT::zero())
                .map(|(literal, coefficient)| (literal.index(), (literal.value(), coefficient))),
        );
        self.lower = linear_constraint.lower();
    }

    pub fn iter_terms_mut(&mut self) -> impl Iterator<Item = (Literal, &mut ValueT)> + '_ {
        return self
            .terms
            .iter_mut()
            .map(|(&index, term)| (Literal::new(index, term.0), &mut term.1));
    }

    pub fn lower_mut(&mut self) -> &mut ValueT {
        &mut self.lower
    }

    pub fn add_assign(&mut self, reason_constraint: impl LinearConstraintTrait<Value = ValueT>) {
        self.lower = self.lower + reason_constraint.lower();
        for (literal, coefficient) in reason_constraint.iter_terms() {
            if let Some(term) = self.terms.get_mut(literal.index()) {
                if term.0 == literal.value() {
                    term.1 = term.1 + coefficient;
                } else {
                    if term.1 > coefficient {
                        self.lower = self.lower - coefficient;
                        term.1 = term.1 - coefficient;
                    } else if term.1 < coefficient {
                        self.lower = self.lower - term.1;
                        term.0 = !term.0;
                        term.1 = coefficient - term.1;
                    } else {
                        self.lower = self.lower - term.1;
                        self.terms.remove(literal.index());
                    }
                }
            } else {
                self.terms.insert(literal.index(), (literal.value(), coefficient));
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LinearConstraintExplainKey {
    row_id: usize,
}

#[derive(Clone, Debug)]
struct Row<ValueT> {
    terms: Vec<(Literal, ValueT)>,
    lower: ValueT,
    is_learnt: bool,
    sup: ValueT,
    max_unassigned_coefficient: ValueT,
}

impl<ValueT> LinearConstraintTrait for &Row<ValueT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.terms.iter().cloned()
    }
    fn lower(&self) -> Self::Value {
        self.lower
    }
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

pub struct LinearConstraintEngine<ValueT, InnerEngineT> {
    state: State<LinearConstraintExplainKey>,
    inner_engine: InnerEngineT,
    rows: Vec<Row<ValueT>>,
    columns: LiteralArray<Column<ValueT>>,
    number_of_confirmed_assignments: usize,
    constraint_queue: VecDeque<(LinearConstraint<ValueT>, bool)>,
    conflict_rows: Set,
    confirming_rows: Set,
}

impl<ValueT, InnerEngineT> LinearConstraintEngine<ValueT, InnerEngineT> {
    pub fn new(inner_engine: InnerEngineT) -> Self {
        Self {
            state: State::Noconflict,
            inner_engine,
            rows: Vec::default(),
            columns: LiteralArray::default(),
            number_of_confirmed_assignments: 0,
            constraint_queue: VecDeque::default(),
            conflict_rows: Set::default(),
            confirming_rows: Set::default(),
        }
    }
}

impl<ValueT, InnerEngineT> Deref for LinearConstraintEngine<ValueT, InnerEngineT>
where
    InnerEngineT: Deref,
{
    type Target = InnerEngineT::Target;
    fn deref(&self) -> &Self::Target {
        self.inner_engine.deref()
    }
}

impl<ValueT, InnerEngineT> EngineTrait for LinearConstraintEngine<ValueT, InnerEngineT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive + Debug,
    InnerEngineT: EngineTrait,
    InnerEngineT::CompositeExplainKey: From<LinearConstraintExplainKey>,
{
    type CompositeExplainKey = InnerEngineT::CompositeExplainKey;
    type ExplainKey = Either<LinearConstraintExplainKey, InnerEngineT::ExplainKey>;
    type ExplanationConstraint<'a>
        = Either<
        impl LinearConstraintTrait<Value = ValueT> + 'a,
        InnerEngineT::ExplanationConstraint<'a>,
    >
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
        // if let Reason::Propagation { explain_key } = reason {
        //     assert!(explain_key.try_into().is_err());
        // }
        self.inner_engine.assign(literal, reason);
        self.propagate();
    }

    fn backjump(&mut self, backjump_level: usize) {
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
}

impl<ValueT, InnerEngineT, LinearConstraintT, InnerConstraintT>
    EngineAddConstraintTrait<Either<LinearConstraintT, InnerConstraintT>>
    for LinearConstraintEngine<ValueT, InnerEngineT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive + Debug,
    LinearConstraintT: LinearConstraintTrait<Value = ValueT>,
    InnerEngineT: EngineTrait + EngineAddConstraintTrait<InnerConstraintT>,
    InnerEngineT::CompositeExplainKey: From<LinearConstraintExplainKey>,
{
    fn add_constraint(
        &mut self,
        constraint: Either<LinearConstraintT, InnerConstraintT>,
        is_learnt: bool,
    ) {
        match constraint {
            // self 用の制約である場合
            Either::Left(constraint) => {
                // CountConstraint を構築
                let mut constraint: LinearConstraint<_> = (&constraint).into();

                // 伝播が発生する最小の決定レベルを特定
                let min_propagation_level = {
                    let mut sup = ValueT::zero();
                    let mut max_unassigned_coefficient = ValueT::zero();
                    for (literal, coefficient) in constraint.terms.iter().cloned() {
                        if !self.inner_engine.is_false(literal) {
                            sup = sup + coefficient;
                        }
                        if !self.inner_engine.is_assigned(literal.index()) {
                            max_unassigned_coefficient =
                                max(max_unassigned_coefficient, coefficient);
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
                            max_unassigned_coefficient =
                                max(max_unassigned_coefficient, coefficient);
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
            // inner_engine 用の制約である場合
            Either::Right(constraint) => {
                self.inner_engine.add_constraint(constraint, is_learnt);
            }
        }

        self.propagate();
    }
}

impl<ValueT, InnerEngineT> LinearConstraintEngine<ValueT, InnerEngineT>
where
    ValueT: Integer + Unsigned + Copy + Debug,
    InnerEngineT: EngineTrait,
    InnerEngineT::CompositeExplainKey: From<LinearConstraintExplainKey>,
{
    fn propagate(&mut self) {
        self.conflict_rows.clear();
        self.confirming_rows.clear();

        while self.state.is_noconflict() && self.inner_engine.state().is_noconflict() {
            if !self.conflict_rows.is_empty() {
                // 矛盾している制約が現れた場合には state を Conflict に
                let &row_id = self.conflict_rows.iter().next().unwrap();
                let row = &self.rows[row_id];
                eprintln!(
                    "CONFLICT {:?} {:?} {:?}",
                    row.sup, row.lower, row.max_unassigned_coefficient
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
            if row.sup < row.lower {
                self.conflict_rows.insert(row_id);
            } else if row.sup < row.lower + row.max_unassigned_coefficient {
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
        debug_assert!(row.sup >= row.lower);
        debug_assert!(row.sup < row.lower + row.max_unassigned_coefficient);
        row.max_unassigned_coefficient = ValueT::zero();
        while k < row.terms.len() {
            let (literal, coefficient) = row.terms[k];
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
        if row.max_unassigned_coefficient > row.sup - row.lower {
            let explain_key = LinearConstraintExplainKey { row_id };
            while k < row.terms.len() {
                let (literal, coefficient) = row.terms[k];
                if row.sup - coefficient < row.lower {
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
            terms: constraint.terms,
            lower: constraint.lower,
            is_learnt,
            sup,
            max_unassigned_coefficient,
        });
        let row = self.rows.last_mut().unwrap();

        // 係数の降順にソート
        row.terms.sort_unstable_by_key(|&(_, coefficient)| Reverse(coefficient));

        // 列に項を追加
        for &(literal, coefficient) in row.terms.iter() {
            self.columns[literal].terms.push((row_id, coefficient));
        }

        if row.sup < row.lower {
            // 違反している場合
            self.conflict_rows.insert(row_id);
        // 伝播が発生する場合
        } else if row.sup - row.max_unassigned_coefficient < row.lower {
            self.confirming_rows.insert(row_id);
        }
    }
}
