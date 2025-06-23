use std::ops::AddAssign;

use num::{Integer, Unsigned};

use crate::{
    Literal, collections::LiteralSet, constraint::LinearConstraintTrait, pb_engine::PBEngine,
};

#[derive(Clone)]
struct Term<ValueT> {
    literal: Literal,
    coefficient: ValueT,
}

#[derive(Default, Clone)]
pub struct IdentifyConflictCausals<ValueT> {
    causal_terms: Vec<Term<ValueT>>,
    calsal_term_set: LiteralSet,
}

impl<ValueT> IdentifyConflictCausals<ValueT>
where
    ValueT: Integer + Unsigned + Copy + AddAssign,
{
    pub fn call<PriorityT: Ord>(
        &mut self,
        conflict_constraint: &impl LinearConstraintTrait<Value = ValueT>,
        conflict_order: usize,
        get_priority: impl Fn(Literal, ValueT) -> PriorityT,
        pb_engine: &PBEngine,
    ) -> (&'_ LiteralSet, ValueT) {
        self.causal_terms.clear();
        let mut sup = ValueT::zero();
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
        if sup + ValueT::one() < conflict_constraint.lower() {
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
        self.calsal_term_set.extend(self.causal_terms.iter().map(|term| !term.literal));
        return (&self.calsal_term_set, slack);
    }
}
