use crate::{
    Literal,
    collections::LiteralSet,
    pb_engine::{LinearConstraintTrait, PBEngine},
};

#[derive(Clone)]
struct Term {
    literal: Literal,
    coefficient: u64,
}

#[derive(Clone)]
pub struct IdentifyPropagationCausals {
    causal_terms: Vec<Term>,
    calsal_term_set: LiteralSet,
}

impl IdentifyPropagationCausals {
    pub fn new() -> Self {
        Self {
            causal_terms: Vec::default(),
            calsal_term_set: LiteralSet::default(),
        }
    }

    pub fn call<PriorityT: Ord>(
        &mut self,
        reason_constraint: &impl LinearConstraintTrait<Value = u64>,
        propagated_literal: Literal,
        get_priority: impl Fn(Literal) -> PriorityT,
        pb_engine: &PBEngine<u64>,
    ) -> (&'_ LiteralSet, u64) {
        let resolving_order = pb_engine.get_assignment_order(propagated_literal.index());

        self.causal_terms.clear();
        let mut sup = 0;
        let mut resolving_coefficient = 0;
        for (literal, coefficient) in reason_constraint.iter_terms() {
            if pb_engine.is_false_at(literal, resolving_order - 1) {
                self.causal_terms.push(Term {
                    literal,
                    coefficient,
                });
            } else {
                sup += coefficient;
            }
            if literal == propagated_literal {
                // debug_assert!(pb_engine.is_true(literal));
                resolving_coefficient = coefficient;
            }
        }
        debug_assert!(sup < reason_constraint.lower() + resolving_coefficient);

        // causal_term から項を除く余地があれば除く
        if sup + 1 < reason_constraint.lower() + resolving_coefficient {
            // 優先度の高い順にソート
            self.causal_terms
                .sort_unstable_by_key(|term| std::cmp::Reverse(get_priority(term.literal)));
            // 後ろから走査
            for k in (0..self.causal_terms.len()).rev() {
                let term = &self.causal_terms[k];
                // term を除いても伝播が発生するなら除去
                if sup + term.coefficient < reason_constraint.lower() + resolving_coefficient {
                    sup += term.coefficient;
                    self.causal_terms.swap_remove(k);
                }
            }
        }
        debug_assert!(sup < reason_constraint.lower() + resolving_coefficient);
        debug_assert!(sup >= reason_constraint.lower());

        let slack = sup - reason_constraint.lower();

        self.calsal_term_set.clear();
        self.calsal_term_set.extend(self.causal_terms.iter().map(|term| !term.literal));

        return (&self.calsal_term_set, slack);
    }
}
