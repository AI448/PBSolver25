use super::etc::{Reason, State};
use crate::{
    Boolean, Literal,
    collections::LiteralArray,
    pb_engine::{
        CountConstraintEngine, CountConstraintExplainKey, CountConstraintTrait,
        CountConstraintView, DecisionStack, MonadicConstraintExplainKey,
        count_constraint_engine::CountConstraintEngineExplainKey,
    },
};
use either::Either;
use num::{FromPrimitive, Integer, Num, NumCast, ToPrimitive, Unsigned, Zero};
use std::{
    cmp::{Reverse, max},
    collections::VecDeque,
    fmt::Debug,
    ops::Deref,
};
use utility::{Map, Set};

pub trait LinearConstraintTrait {
    type Value: Integer + Unsigned + Copy + FromPrimitive + Zero;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone;
    fn lower(&self) -> Self::Value;

    fn mul(&self, multipler: Self::Value) -> impl LinearConstraintTrait<Value = Self::Value> {
        return LinearConstraintView::new(
            self.iter_terms().map(move |(literal, coefficient)| (literal, coefficient * multipler)),
            self.lower() * multipler,
        );
    }

    fn as_view(&self) -> impl LinearConstraintTrait<Value = Self::Value> {
        return LinearConstraintView::new(self.iter_terms(), self.lower());
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

    fn drop_fixed_variables<ExplainKeyT>(
        &self,
        decision_stack: &DecisionStack<ExplainKeyT>,
    ) -> impl LinearConstraintTrait<Value = Self::Value> {
        let mut lower = self.lower();
        for (literal, coefficient) in self.iter_terms() {
            if decision_stack.is_true(literal)
                && decision_stack.get_decision_level(literal.index()) == 0
            {
                if lower < coefficient {
                    lower = Self::Value::zero();
                    break;
                } else {
                    lower = lower - coefficient;
                }
            }
        }
        return LinearConstraintView::new(
            self.iter_terms().filter(|&(literal, coefficient)| {
                !coefficient.is_zero() && decision_stack.get_decision_level(literal.index()) != 0
            }),
            lower,
        );
    }
}

// impl<LinearConstraintT> LinearConstraintTrait for &LinearConstraintT
// where
//     LinearConstraintT: LinearConstraintTrait,
// {
//     type Value = LinearConstraintT::Value;
//     fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
//         (*self).iter_terms()
//     }

//     fn lower(&self) -> Self::Value {
//         (*self).lower()
//     }
// }

impl<CountConstraintT> LinearConstraintTrait for CountConstraintT
where
    CountConstraintT: CountConstraintTrait,
{
    type Value = u64;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.iter_terms().map(|literal| (literal, 1))
    }
    fn lower(&self) -> Self::Value {
        self.lower() as Self::Value
    }
}

pub enum CompositeLinearConstraint<LeftLinearConstraintT, RightLinearConstraintT> {
    Left(LeftLinearConstraintT),
    Right(RightLinearConstraintT),
}

impl<LeftLinearConstraintT, RightLinearConstraintT, ValueT> LinearConstraintTrait
    for CompositeLinearConstraint<LeftLinearConstraintT, RightLinearConstraintT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive,
    LeftLinearConstraintT: LinearConstraintTrait<Value = ValueT>,
    RightLinearConstraintT: LinearConstraintTrait<Value = ValueT>,
{
    type Value = ValueT;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        return match self {
            Self::Left(left) => Either::Left(left.iter_terms()),
            Self::Right(right) => Either::Right(right.iter_terms()),
        };
    }

    fn lower(&self) -> Self::Value {
        return match self {
            Self::Left(left) => left.lower(),
            Self::Right(right) => right.lower(),
        };
    }
}

// impl<LeftLinearConstraintT, RightLinearConstraintT, ValueT> LinearConstraintTrait
//     for Either<LeftLinearConstraintT, RightLinearConstraintT>
// where
//     ValueT: Integer + Unsigned + Copy + FromPrimitive,
//     LeftLinearConstraintT: LinearConstraintTrait<Value = ValueT>,
//     RightLinearConstraintT: LinearConstraintTrait<Value = ValueT>,
// {
//     type Value = ValueT;
//     fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
//         return match self {
//             Either::Left(left) => Either::Left(left.iter_terms()),
//             Either::Right(right) => Either::Right(right.iter_terms()),
//         };
//     }
//     fn lower(&self) -> Self::Value {
//         return match self {
//             Either::Left(left) => left.lower(),
//             Either::Right(right) => right.lower(),
//         };
//     }
// }

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
        linear_constraint: &impl LinearConstraintTrait<Value = ValueT>,
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

    pub fn add_assign(&mut self, reason_constraint: &impl LinearConstraintTrait<Value = ValueT>) {
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

pub struct StrengthenLinearConstraint<ValueT> {
    terms: Vec<(Literal, ValueT)>,
}

impl<ValueT> Default for StrengthenLinearConstraint<ValueT> {
    fn default() -> Self {
        Self {
            terms: Vec::default(),
        }
    }
}

impl<ValueT> Clone for StrengthenLinearConstraint<ValueT> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<ValueT> StrengthenLinearConstraint<ValueT>
where
    ValueT: Integer + Unsigned + Copy + FromPrimitive,
{
    fn exec<ExplainKeyT>(
        &mut self,
        constraint: &impl LinearConstraintTrait<Value = ValueT>,
        decision_stack: &DecisionStack<ExplainKeyT>,
    ) -> impl LinearConstraintTrait<Value = ValueT> + '_ {
        // 決定レベル 0 で割り当てられている変数を除いて制約を複製
        self.terms.clear();
        let mut lower = constraint.lower();
        for (literal, coefficient) in constraint.iter_terms() {
            if decision_stack.get_decision_level(literal.index()) == 0 {
                if decision_stack.is_true(literal) {
                    if lower <= coefficient {
                        // 右辺値が 0 になる場合には自明に満たされる制約条件なので両辺を 0 にして中断
                        self.terms.clear();
                        lower = ValueT::zero();
                        break;
                    } else {
                        lower = lower - coefficient;
                    }
                }
            } else {
                self.terms.push((literal, coefficient));
            }
        }
        // lower より大きい係数を lower に一致させる
        for (_, coefficient) in self.terms.iter_mut() {
            if *coefficient > lower {
                *coefficient = lower;
            }
        }
        // 係数が 0 である項を除く
        self.terms.retain(|&(_, coefficient)| coefficient > ValueT::zero());
        // 係数の最大公約数を使って丸め
        if self.terms.len() >= 1 {
            let mut gcd = ValueT::zero();
            for (_, coefficient) in self.terms.iter() {
                gcd = gcd.gcd(coefficient);
            }
            for (_, coefficient) in self.terms.iter_mut() {
                debug_assert!(*coefficient % gcd == ValueT::zero());
                *coefficient = *coefficient / gcd;
            }
            lower = lower.div_ceil(&gcd);
        }
        // 下限より小さい係数の合計を算出
        let mut sum_of_unsaturating_coefficients = ValueT::zero();
        for (_, coefficient) in self.terms.iter_mut() {
            if *coefficient < lower {
                sum_of_unsaturating_coefficients = sum_of_unsaturating_coefficients + *coefficient;
            }
        }
        // 下限より小さい係数の合計が下限未満であればそれらは 0 に切り下げることができる
        if sum_of_unsaturating_coefficients < lower {
            self.terms.retain(|&(_, coefficient)| coefficient == lower);
            for (_, coefficient) in self.terms.iter_mut() {
                *coefficient = ValueT::one();
            }
            lower = ValueT::one();
        }
        return LinearConstraintView::new(self.terms.iter().cloned(), lower);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LinearConstraintExplainKey {
    row_id: usize,
}

#[derive(Clone, Debug)]
struct Row {
    terms: Vec<(Literal, u64)>,
    lower: u64,
    is_learnt: bool,
    sup: u64,
    max_unassigned_coefficient: u64,
}

impl LinearConstraintTrait for &Row {
    type Value = u64;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, Self::Value)> + Clone {
        self.terms.iter().cloned()
    }
    fn lower(&self) -> Self::Value {
        self.lower
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LinearConstraintEngineExplainKey {
    LinearConstraint(LinearConstraintExplainKey),
    CountConstraint(CountConstraintEngineExplainKey),
}

impl From<LinearConstraintExplainKey> for LinearConstraintEngineExplainKey {
    fn from(explain_key: LinearConstraintExplainKey) -> Self {
        Self::LinearConstraint(explain_key)
    }
}

// impl From<CountConstraintEngineExplainKey> for LinearConstraintEngineExplainKey {
//     fn from(explain_key: CountConstraintEngineExplainKey) -> Self {
//         Self::CountConstraint(explain_key)
//     }
// }

impl<T> From<T> for LinearConstraintEngineExplainKey
where
    T: Into<CountConstraintEngineExplainKey>,
{
    fn from(value: T) -> Self {
        Self::CountConstraint(value.into())
    }
}

#[derive(Default, Clone, Debug)]
struct Column {
    terms: Vec<(usize, u64)>,
}

pub struct LinearConstraintEngine<CompositeExplainKeyT> {
    strengthen: StrengthenLinearConstraint<u64>,
    state: State<LinearConstraintExplainKey>,
    inner_engine: CountConstraintEngine<CompositeExplainKeyT>,
    rows: Vec<Row>,
    columns: LiteralArray<Column>,
    number_of_confirmed_assignments: usize,
    constraint_queue: VecDeque<(LinearConstraint<u64>, bool)>,
    conflict_rows: Set,
    confirming_rows: Set,
}

impl<CompositeExplainKeyT> LinearConstraintEngine<CompositeExplainKeyT> {
    pub fn new() -> Self {
        Self {
            strengthen: StrengthenLinearConstraint::default(),
            state: State::Noconflict,
            inner_engine: CountConstraintEngine::new(),
            rows: Vec::default(),
            columns: LiteralArray::default(),
            number_of_confirmed_assignments: 0,
            constraint_queue: VecDeque::default(),
            conflict_rows: Set::default(),
            confirming_rows: Set::default(),
        }
    }
}

impl<CompositeExplainKeyT> Deref for LinearConstraintEngine<CompositeExplainKeyT> {
    type Target = DecisionStack<CompositeExplainKeyT>;
    fn deref(&self) -> &Self::Target {
        self.inner_engine.deref()
    }
}

impl<CompositeExplainKeyT> LinearConstraintEngine<CompositeExplainKeyT>
where
    CompositeExplainKeyT: From<LinearConstraintExplainKey>
        + From<CountConstraintExplainKey>
        + From<MonadicConstraintExplainKey>,
{
    pub fn state(&self) -> State<LinearConstraintEngineExplainKey> {
        return self.state.composite(self.inner_engine.state());
    }

    pub fn explain(
        &self,
        explain_key: LinearConstraintEngineExplainKey,
    ) -> impl LinearConstraintTrait<Value = u64> {
        return match explain_key {
            LinearConstraintEngineExplainKey::LinearConstraint(explain_key) => {
                CompositeLinearConstraint::Left(&self.rows[explain_key.row_id])
            }
            LinearConstraintEngineExplainKey::CountConstraint(explain_key) => {
                CompositeLinearConstraint::Right(self.inner_engine.explain(explain_key))
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
        constraint: &impl LinearConstraintTrait<Value = u64>,
        is_learnt: bool,
    ) {
        {
            // for (literal, coefficient) in constraint.iter_terms() {
            //     eprint!("{} {} ", literal, coefficient);
            // }
            // eprintln!(">= {}", constraint.lower());
            // 制約を強化
            let constraint = self.strengthen.exec(constraint, &self.inner_engine);
            // for (literal, coefficient) in constraint.iter_terms() {
            //     eprint!("{} {} ", literal, coefficient);
            // }
            // eprintln!(">= {}", constraint.lower());
            // 恒等的に充足される制約であれば追加しない
            if constraint.lower() == 0 {
                return;
            }
            if constraint.iter_terms().all(|(_, coefficient)| coefficient == 1) {
                // すべての係数が 1 である場合には inner_engine に追加する
                self.inner_engine.add_constraint(
                    &CountConstraintView::new(
                        constraint.iter_terms().map(|(literal, _)| literal),
                        constraint.lower() as usize,
                    ),
                    is_learnt,
                );
            } else {
                // そうでなければ伝播の発生を確認して状態を更新し，制約をキューに追加
                let mut constraint: LinearConstraint<_> = (&constraint).into();
                // 伝播が発生する最小の決定レベルを特定
                // TODO: 無駄に浅い階層までバックジャンプすることがあるので，CalculatePropagationLevel を使う
                let min_propagation_level = {
                    // 現在の上界と未割り当て変数の係数の最大値を算出
                    let mut sup = 0;
                    let mut max_unassigned_coefficient = 0;
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
        row.max_unassigned_coefficient = 0;
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
            let mut sup_at_previous_decision_level = 0;
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
        let mut sup = 0;
        let mut max_unassigned_coefficient = 0;
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
