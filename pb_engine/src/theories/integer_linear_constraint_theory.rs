use std::cmp::max;

use super::{Propagation, TheoryAddConstraintTrait, TheoryTrait};
use crate::{
    Literal, collections::LiteralArray, constraints::LinearConstraintTrait,
    decision_stack::DecisionStack,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IntegerLinearConstraintExplainKey {
    row_id: usize,
}

#[derive(Clone)]
pub struct IntegerLinearConstraintTheory {
    rows: Vec<Row>,
    columns: LiteralArray<Column>,
    number_of_evaluated_assignments: usize,
}

impl IntegerLinearConstraintTheory {
    pub fn new() -> Self {
        Self {
            rows: Vec::default(),
            columns: LiteralArray::default(),
            number_of_evaluated_assignments: 0,
        }
    }

    pub fn number_of_constraints(&self) -> usize {
        // TODO: 学習節の削除を行うとずれるのでちゃんと数える
        return self.rows.len();
    }
}

impl TheoryTrait for IntegerLinearConstraintTheory {
    type ExplainKey = IntegerLinearConstraintExplainKey;
    type ExplanationConstraint<'a> = impl LinearConstraintTrait<Value = u64> + 'a;

    fn add_variable(&mut self) {
        self.columns.push([Column::default(), Column::default()]);
    }

    fn assign<ExplainKeyT: Copy>(
        &mut self,
        decision_stack: &DecisionStack<ExplainKeyT>,
        mut callback: impl FnMut(Propagation<Self::ExplainKey>),
    ) {
        assert!(decision_stack.number_of_assignments() == self.number_of_evaluated_assignments + 1);
        let assigned_literal = decision_stack.get_assignment(self.number_of_evaluated_assignments);
        self.number_of_evaluated_assignments += 1;
        // !assigned_literal を含む行を走査
        for &(row_id, coefficient) in self.columns[!assigned_literal].terms.iter() {
            let row = &mut self.rows[row_id];
            // 左辺値の上界を更新
            row.sup -= coefficient;
            debug_assert!(row.sup >= row.lower);
            // 伝播が発生する可能性がなければ continue
            if row.sup >= row.lower + row.max_unassigned_coefficient {
                continue;
            }
            // 左辺の項を先頭から走査
            // NOTE: 係数が降順にソートされていることを前提としている
            let mut k = 0;
            // 未割り当てのリテラルの係数の最大値を更新
            row.max_unassigned_coefficient = 0;
            while k < row.terms.len() {
                let (literal, coefficient) = row.terms[k];
                if !decision_stack.is_assigned(literal.index()) {
                    row.max_unassigned_coefficient = coefficient;
                    break;
                }
                k += 1;
            }
            // 伝播
            while k < row.terms.len() {
                let (literal, coefficient) = row.terms[k];
                // 伝播が発生しない場合には break
                if row.sup >= row.lower + coefficient {
                    break;
                }
                // 未割り当てであれば伝播
                if !decision_stack.is_assigned(literal.index()) {
                    callback(Propagation {
                        literal,
                        explain_key: IntegerLinearConstraintExplainKey { row_id },
                    });
                }
                k += 1;
            }
        }
    }

    fn backjump<ExplainKeyT: Copy>(
        &mut self,
        backjump_level: usize,
        decision_stack: &DecisionStack<ExplainKeyT>,
    ) {
        let backjump_order = decision_stack.order_range(backjump_level).end;
        assert!(backjump_order <= self.number_of_evaluated_assignments);
        while backjump_order < self.number_of_evaluated_assignments {
            self.number_of_evaluated_assignments -= 1;
            // 未割り当てになるリテラルを取得
            let unassigned_literal =
                decision_stack.get_assignment(self.number_of_evaluated_assignments);
            // !unassigned_literal を含む制約条件の左辺値の上界と，未割り当てリテラルの係数の最大値を更新
            for &(row_id, coefficient) in self.columns[!unassigned_literal].terms.iter() {
                let row = &mut self.rows[row_id];
                row.sup += coefficient;
                assert!(row.sup >= row.lower);
                row.max_unassigned_coefficient =
                    u64::max(row.max_unassigned_coefficient, coefficient);
            }
            // unassigned_literal を含む制約条件の未割り当てリテラルの係数の最大値を更新
            for &(row_id, coefficient) in self.columns[unassigned_literal].terms.iter() {
                let row = &mut self.rows[row_id];
                row.max_unassigned_coefficient =
                    u64::max(row.max_unassigned_coefficient, coefficient);
            }
        }
    }

    fn explain(&self, explain_key: Self::ExplainKey) -> Self::ExplanationConstraint<'_> {
        return &self.rows[explain_key.row_id];
    }
}

impl<ConstraintT> TheoryAddConstraintTrait<ConstraintT> for IntegerLinearConstraintTheory
where
    ConstraintT: LinearConstraintTrait<Value = u64>,
{
    fn add_constraint<ExplainKeyT: Copy>(
        &mut self,
        constraint: ConstraintT,
        decision_stack: &DecisionStack<ExplainKeyT>,
        mut callback: impl FnMut(Propagation<Self::ExplainKey>),
    ) -> Result<(), usize> {
        if constraint.lower() <= 0 {
            return Ok(());
        }

        let mut terms = Vec::from_iter(constraint.iter_terms());
        let lower = constraint.lower();

        // 係数の降順にソート
        terms.sort_unstable_by(|l, r| r.1.partial_cmp(&l.1).unwrap());

        // 左辺値の上界と未割り当てリテラルの係数の最大値を算出
        let mut sup = 0;
        let mut max_unassigned_coefficient = 0;
        let mut sup_at_previous_decision_level = 0;
        for &(literal, coefficient) in terms.iter().rev() {
            if !decision_stack.is_false(literal) {
                sup += coefficient;
            }
            if !decision_stack.is_assigned(literal.index()) {
                max_unassigned_coefficient = max(max_unassigned_coefficient, coefficient);
            }
            if !(decision_stack.is_false(literal)
                && decision_stack.get_decision_level(literal.index())
                    < decision_stack.decision_level())
            {
                sup_at_previous_decision_level += coefficient;
            }
        }
        assert!(sup >= lower);
        // TODO Err を返す
        assert!(decision_stack.decision_level() == 0 || sup_at_previous_decision_level >= lower + max_unassigned_coefficient,
            "{} {} {}",
            sup_at_previous_decision_level, lower, max_unassigned_coefficient
        );

        // 制約条件を追加
        let row_id = self.rows.len();
        self.rows.push(Row {
            terms,
            lower,
            sup,
            max_unassigned_coefficient,
        });
        let row = self.rows.last().unwrap();

        // 列方向の係数を追加
        for &(literal, coefficient) in row.terms.iter() {
            self.columns[literal].terms.push((row_id, coefficient));
        }

        // 追加した制約条件による伝播を実行
        if sup < lower + max_unassigned_coefficient {
            for &(literal, coefficient) in row.terms.iter() {
                // 伝播が発生しない場合には break
                if row.sup >= row.lower + coefficient {
                    break;
                }
                // 未割り当てであれば伝播
                if !decision_stack.is_assigned(literal.index()) {
                    callback(Propagation {
                        literal,
                        explain_key: IntegerLinearConstraintExplainKey { row_id },
                    });
                }
            }
        }

        return Ok(());
    }
}

#[derive(Clone, Debug)]
struct Row {
    terms: Vec<(Literal, u64)>,
    lower: u64,
    sup: u64,
    max_unassigned_coefficient: u64,
}

impl LinearConstraintTrait for Row {
    type Value = u64;
    fn iter_terms(&self) -> impl Iterator<Item = (Literal, u64)> + Clone + '_ {
        self.terms.iter().cloned()
    }

    fn lower(&self) -> u64 {
        self.lower
    }
}

#[derive(Default, Clone, Debug)]
struct Column {
    terms: Vec<(usize, u64)>,
}
