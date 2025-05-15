use crate::{LinearConstraintTrait, Literal, PBEngine, collections::LiteralSet};

#[derive(Clone)]
struct Term {
    literal: Literal,
    coefficient: u64,
}

#[derive(Default, Clone)]
pub struct IdentifyConflictCausals {
    causal_terms: Vec<Term>,
    calsal_term_set: LiteralSet,
}

impl IdentifyConflictCausals {
    pub fn call<PriorityT: Ord>(
        &mut self,
        conflict_constraint: &impl LinearConstraintTrait<Value = u64>,
        conflict_order: usize,
        get_priority: impl Fn(Literal, u64) -> PriorityT,
        pb_engine: &PBEngine,
    ) -> (&'_ LiteralSet, u64) {
        self.causal_terms.clear();
        let mut sup = 0;
        for (literal, coefficient) in conflict_constraint.iter_terms() {
            if pb_engine.is_false_at(literal, conflict_order) {
                self.causal_terms.push(Term {
                    literal,
                    coefficient,
                });
            } else {
                sup += coefficient;
            }
        }
        debug_assert!(sup < conflict_constraint.lower());

        // causal_term から項を除く余地があれば除く
        if sup + 1 < conflict_constraint.lower() {
            // 優先度の高い順にソート
            self.causal_terms.sort_unstable_by_key(|term| {
                std::cmp::Reverse(get_priority(term.literal, term.coefficient))
            });
            // 後ろから走査
            for k in (0..self.causal_terms.len()).rev() {
                let term = &self.causal_terms[k];
                // term を除いても伝播が発生するなら除去
                if sup + term.coefficient < conflict_constraint.lower() {
                    sup += term.coefficient;
                    self.causal_terms.swap_remove(k);
                }
            }
        }
        debug_assert!(sup < conflict_constraint.lower());

        let slack = conflict_constraint.lower() - sup;

        self.calsal_term_set.clear();
        self.calsal_term_set
            .extend(self.causal_terms.iter().map(|term| !term.literal));
        return (&self.calsal_term_set, slack);
    }
}
