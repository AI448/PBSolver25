use crate::{LinearConstraintTrait, Literal, PBEngine};



struct Term {
    literal: Literal,
    coefficient: u64,
    order: usize,
}

#[derive(Default)]
pub struct FindConflictLiteral {
    falsified_literals: Vec<Term>
}

impl Clone for FindConflictLiteral {
    fn clone(&self) -> Self {
        Self::default()    
    }
}

impl FindConflictLiteral {
    pub fn find(&mut self, conflict_constraint: impl LinearConstraintTrait<Value = u64>, engine: &PBEngine) -> Literal {
        self.falsified_literals.clear();
        let mut sup = 0;
        for (literal, coefficient) in conflict_constraint.iter_terms() {
            if engine.is_false(literal) {
                let order = engine.get_assignment_order(literal.index());
                self.falsified_literals.push(Term { literal, coefficient, order });
            } else {
                sup += coefficient;
            }
        }
        debug_assert!(sup < conflict_constraint.lower());

        self.falsified_literals.sort_unstable_by_key(|term| term.order);
        
        while self.falsified_literals.last().is_some_and(|term| sup + term.coefficient < conflict_constraint.lower()) {
            sup += self.falsified_literals.pop().unwrap().coefficient;
        }

        return self.falsified_literals.last().unwrap().literal;
    }
}