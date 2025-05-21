use std::cmp::{max, min};

use crate::{
    Literal, calculate_plbd::CalculatePLBD, collections::LiteralArray,
    constraints::CountConstraintTrait, decision_stack::DecisionStack, theories::Propagation,
};

use super::{TheoryAddConstraintTrait, TheoryTrait};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CountConstraintExplainKey {
    row_id: usize,
}

#[derive(Clone)]
pub struct CountConstraintTheory {
    activity_time_constant: f64,
    calculate_plbd: CalculatePLBD,
    watching_rows: LiteralArray<Vec<Watch>>,
    rows: Vec<Row>,
    number_of_constraints: usize,
    number_of_evaluated_assignments: usize,
    activity_increase_value: f64,
    backjump_count: usize,
    reducing_backjump_count: usize,
}

impl CountConstraintTheory {
    pub fn new(activity_time_constant: f64) -> Self {
        Self {
            activity_time_constant,
            calculate_plbd: CalculatePLBD::default(),
            watching_rows: LiteralArray::default(),
            rows: Vec::default(),
            number_of_constraints: 0,
            number_of_evaluated_assignments: 0,
            activity_increase_value: 1.0,
            backjump_count: 0,
            reducing_backjump_count: 10000,
        }
    }

    pub fn number_of_constraints(&self) -> usize {
        return self.number_of_constraints;
    }
}

impl TheoryTrait for CountConstraintTheory {
    type ExplainKey = CountConstraintExplainKey;
    type ExplanationConstraint<'a> = impl CountConstraintTrait + 'a;

    fn add_variable(&mut self) {
        self.watching_rows.push([Vec::default(), Vec::default()]);
    }

    fn assign<ExplainKeyT: Copy>(
        &mut self,
        decision_stack: &DecisionStack<ExplainKeyT>,
        mut callback: impl FnMut(super::Propagation<Self::ExplainKey>),
    ) {
        assert!(decision_stack.number_of_assignments() <= self.number_of_evaluated_assignments + 1);

        if decision_stack.number_of_assignments() == self.number_of_evaluated_assignments {
            return;
        }

        let assigned_literal = decision_stack.get_assignment(self.number_of_evaluated_assignments);
        self.number_of_evaluated_assignments += 1;

        'for_k: for k in (0..self.watching_rows[!assigned_literal].len()).rev() {
            let watch = self.watching_rows[!assigned_literal][k];
            let row = &mut self.rows[watch.row_id];
            if row.state == RowState::Deleted {
                self.watching_rows[!assigned_literal].swap_remove(k);
                continue;
            }
            debug_assert!(watch.position < row.number_of_watching_literals);
            debug_assert!(row.literals[watch.position] == !assigned_literal);

            for p in row.number_of_watching_literals..row.literals.len() {
                let literal = row.literals[p];
                if !decision_stack.is_false(literal) {
                    row.literals.swap(watch.position, p);
                    self.watching_rows[!assigned_literal].swap_remove(k);
                    self.watching_rows[literal].push(watch);
                    continue 'for_k;
                }
            }

            let plbd = self.calculate_plbd.calculate(
                [assigned_literal].into_iter().chain(
                    row.literals[row.number_of_watching_literals..]
                        .iter()
                        .map(|&literal| !literal),
                ),
                decision_stack,
            );
            row.plbd = min(row.plbd, plbd);
            row.activity += self.activity_increase_value;

            for &literal in row.literals[..row.number_of_watching_literals].iter() {
                debug_assert!(literal == !assigned_literal || !decision_stack.is_false(literal));
                if !decision_stack.is_assigned(literal.index()) {
                    callback(Propagation {
                        literal,
                        explain_key: Self::ExplainKey {
                            row_id: watch.row_id,
                        },
                        plbd,
                    });
                }
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
        self.number_of_evaluated_assignments = backjump_order;
        self.backjump_count += 1;
        self.activity_increase_value /= 1.0 - 1.0 / self.activity_time_constant;

        if backjump_level == 0 && self.backjump_count > self.reducing_backjump_count {
            eprintln!("REDUCE");
            self.reducing_backjump_count = self.backjump_count + 10000 + self.backjump_count / 10;
            let mut rows = Vec::default();
            for (row_id, row) in self.rows.iter_mut().enumerate() {
                row.activity /= self.activity_increase_value;
                if row.state == RowState::Learnt && row.plbd >= 2 {
                    rows.push((row_id, row.activity));
                }
            }
            self.activity_increase_value = 1.0;
            rows.sort_unstable_by(|lhs, rhs| rhs.1.partial_cmp(&lhs.1).unwrap());
            for &(row_id, _) in rows.iter().skip(max(1000, rows.len() / 2)) {
                let row = &mut self.rows[row_id];
                debug_assert!(row.state == RowState::Learnt);
                row.state = RowState::Deleted;
                row.literals.clear();
                self.number_of_constraints -= 1;
            }
        }
    }

    fn explain(&self, explain_key: Self::ExplainKey) -> Self::ExplanationConstraint<'_> {
        return &self.rows[explain_key.row_id];
    }
}

impl<CountConstraintT> TheoryAddConstraintTrait<CountConstraintT> for CountConstraintTheory
where
    CountConstraintT: CountConstraintTrait,
{
    fn add_constraint<ExplainKeyT: Copy>(
        &mut self,
        constraint: CountConstraintT,
        is_learnt: bool,
        decision_stack: &DecisionStack<ExplainKeyT>,
        mut callback: impl FnMut(Propagation<Self::ExplainKey>),
    ) -> Result<(), usize> {
        if constraint.lower() == 0 {
            return Ok(());
        }

        let mut literals = Vec::from_iter(constraint.iter_terms());
        let lower = constraint.lower();

        if (lower as usize) < constraint.len() {
            let number_of_watching_literals = lower as usize + 1;
            // False が割り当てられているリテラルを後ろに寄せる
            {
                let mut i = 0;
                for j in 0..literals.len() {
                    let literal = literals[j];
                    if !decision_stack.is_false(literal) {
                        if i != j {
                            literals.swap(i, j);
                        }
                        i += 1;
                    }
                }
            }
            // TODO 後で考える
            assert!(!decision_stack.is_false(literals[number_of_watching_literals - 2]));

            // 伝播が発生する状態である場合には，最後に False が割り当てられたリテラルを監視範囲の末尾に移動する

            if decision_stack.is_false(literals[number_of_watching_literals - 1]) {
                // 最後に割り当てられたリテラルの位置を取得
                let p = ((number_of_watching_literals - 1)..literals.len())
                    .max_by_key(|&p| decision_stack.get_assignment_order(literals[p].index()))
                    .unwrap();

                // 伝播が発生する決定レベルを確認
                let propagation_level = decision_stack.get_decision_level(literals[p].index());
                if propagation_level < decision_stack.decision_level() {
                    // 現在の決定レベルより前に伝播が発生するならエラー
                    return Err(propagation_level);
                }

                // 移動
                literals.swap(number_of_watching_literals - 1, p);
            }

            // 制約を追加
            let row_id = self.rows.len();
            {
                let plbd = literals.len() - lower as usize;
                self.rows.push(Row {
                    literals,
                    lower,
                    number_of_watching_literals,
                    state: if is_learnt {
                        RowState::Learnt
                    } else {
                        RowState::Original
                    },
                    activity: 0.0,
                    plbd,
                });
            }
            self.number_of_constraints += 1;
            let row = self.rows.last_mut().unwrap();

            // 監視を追加
            for (position, &literal) in row.literals[..number_of_watching_literals]
                .iter()
                .enumerate()
            {
                self.watching_rows[literal].push(Watch { row_id, position });
            }

            // 伝播
            if decision_stack.is_false(row.literals[number_of_watching_literals - 1]) {
                let plbd = self.calculate_plbd.calculate(
                    row.literals[(row.number_of_watching_literals - 1)..]
                        .iter()
                        .map(|&literal| !literal),
                    decision_stack,
                );
                row.plbd = plbd;
                row.activity += self.activity_increase_value;

                for &literal in row.literals[..row.number_of_watching_literals - 1].iter() {
                    debug_assert!(!decision_stack.is_false(literal));
                    if !decision_stack.is_assigned(literal.index()) {
                        callback(Propagation {
                            literal,
                            explain_key: Self::ExplainKey { row_id },
                            plbd,
                        });
                    }
                }
            }
        } else {
            assert!((constraint.lower() as usize) == constraint.len());
            assert!(decision_stack.decision_level() == 0);

            let row_id = self.rows.len();
            self.rows.push(Row {
                literals: constraint.iter_terms().collect(),
                lower,
                number_of_watching_literals: 0,
                state: if is_learnt {
                    RowState::Learnt
                } else {
                    RowState::Original
                },
                activity: 0.0,
                plbd: 0,
            });
            self.number_of_constraints += 1;
            // let row = self.rows.last_mut().unwrap();

            for literal in constraint.iter_terms() {
                debug_assert!(!decision_stack.is_false(literal));
                if !decision_stack.is_assigned(literal.index()) {
                    callback(Propagation {
                        literal,
                        explain_key: Self::ExplainKey { row_id },
                        plbd: 0,
                    });
                }
            }
        }
        return Ok(());
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum RowState {
    Original,
    Learnt,
    Deleted,
}

#[derive(Clone, Debug)]
struct Row {
    literals: Vec<Literal>,
    lower: u64,
    number_of_watching_literals: usize,
    state: RowState,
    activity: f64,
    plbd: usize,
}

impl CountConstraintTrait for Row {
    fn iter_terms(&self) -> impl Iterator<Item = Literal> + Clone + '_ {
        self.literals.iter().cloned()
    }

    fn lower(&self) -> u64 {
        self.lower
    }
}

#[derive(Clone, Copy, Debug)]
struct Watch {
    row_id: usize,
    position: usize,
}
