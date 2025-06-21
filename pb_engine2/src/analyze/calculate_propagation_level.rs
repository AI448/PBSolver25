use utility::Map;

use crate::pb_engine::{LinearConstraintTrait, PBEngine};

#[derive(Clone, Copy)]
struct State {
    sup: u64,
    max_interval: u64,
}

pub struct CalculatePropagationLevel {
    decision_level_to_difference: Map<State>,
    decision_level_and_state: Vec<(usize, State)>,
}

impl CalculatePropagationLevel {
    pub fn new() -> Self {
        Self {
            decision_level_to_difference: Map::default(),
            decision_level_and_state: Vec::default(),
        }
    }

    pub fn call(
        &mut self,
        linear_constraint: &impl LinearConstraintTrait<Value = u64>,
        engine: &PBEngine,
        include_nonfalsified_literals: bool,
    ) -> Option<usize> {
        // 各決定レベルでの上界の減少量と，割り当てられた変数の係数の最大値を算出
        self.decision_level_to_difference.clear();
        self.decision_level_to_difference.insert(
            0,
            State {
                sup: 0,
                max_interval: 0,
            },
        );
        for (literal, coefficient) in linear_constraint.iter_terms() {
            if engine.is_false(literal) {
                let decision_level = engine.get_decision_level(literal.index());
                if let Some(difference) = self.decision_level_to_difference.get_mut(decision_level)
                {
                    difference.sup += coefficient;
                    difference.max_interval = difference.max_interval.max(coefficient);
                } else {
                    self.decision_level_to_difference.insert(
                        decision_level,
                        State {
                            sup: coefficient,
                            max_interval: coefficient,
                        },
                    );
                }
            } else if include_nonfalsified_literals && engine.is_true(literal) {
                let decision_level = engine.get_decision_level(literal.index());
                if let Some(difference) = self.decision_level_to_difference.get_mut(decision_level)
                {
                    difference.max_interval = difference.max_interval.max(coefficient);
                } else {
                    self.decision_level_to_difference.insert(
                        decision_level,
                        State {
                            sup: 0,
                            max_interval: coefficient,
                        },
                    );
                }
            }
        }

        // Map から Array にコピーして決定レベル順にソート
        self.decision_level_and_state.clear();
        self.decision_level_and_state.extend(
            self.decision_level_to_difference
                .iter()
                .map(|(decision_level, difference)| (*decision_level, difference.clone())),
        );
        self.decision_level_and_state.sort_unstable_by_key(|(decision_level, _)| *decision_level);

        // 各決定レベルでの左辺値の上界を算出
        {
            let mut sup = linear_constraint.iter_terms().map(|(_, c)| c).sum::<u64>();
            for i in 0..self.decision_level_and_state.len() {
                sup -= self.decision_level_and_state[i].1.sup;
                self.decision_level_and_state[i].1.sup = sup;
            }
        }

        // 各決定レベルでの未割り当てリテラルの係数の最大値を算出
        {
            let mut max_interval = linear_constraint
                .iter_terms()
                .filter(|(l, _)| !engine.is_assigned(l.index()))
                .map(|(_, c)| c)
                .max_by(|l, r| l.partial_cmp(r).unwrap())
                .unwrap_or(0);
            for i in (0..self.decision_level_and_state.len()).rev() {
                let previous_max_interval =
                    max_interval.max(self.decision_level_and_state[i].1.max_interval);
                self.decision_level_and_state[i].1.max_interval = max_interval;
                max_interval = previous_max_interval;
            }
        }

        // 伝播が発生する決定レベルを特定
        // NOTE: 伝播が発生する決定レベルは複数存在し得るが，最も小さいものを返す
        let lower = linear_constraint.lower();
        // eprintln!("{}", lower);
        for &(decision_level, state) in self.decision_level_and_state.iter() {
            debug_assert!(state.sup >= state.max_interval);
            if state.sup < lower {
                return None;
            }
            if state.sup - state.max_interval < lower {
                return Some(decision_level);
            }
        }
        return None;
    }
}
