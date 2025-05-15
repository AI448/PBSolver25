use std::hint::unreachable_unchecked;

use utility::Set;

use crate::types::{Boolean, Literal};

#[derive(Default, Clone)]
pub struct LiteralSet {
    set: Set,
}

impl LiteralSet {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.set.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    #[inline(always)]
    pub fn contains_key(&self, literal: Literal) -> bool {
        self.set.contains_key(literal.bits())
    }

    pub fn is_subset_of(&self, literals: impl Iterator<Item = Literal>) -> bool {
        literals
            .filter(|literal| self.contains_key(*literal))
            .count()
            == self.len()
    }

    #[inline(always)]
    pub fn insert(&mut self, literal: Literal) {
        self.set.insert(literal.bits());
    }

    pub fn extend(&mut self, literals: impl Iterator<Item = Literal>) {
        for literal in literals {
            self.insert(literal);
        }
    }

    #[inline(always)]
    pub fn remove(&mut self, literal: Literal) {
        self.set.remove(literal.bits());
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.set.clear();
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = Literal> + Clone + '_ {
        self.set.iter().map(|bits| {
            Literal::new(
                bits >> 1,
                match bits & 1 {
                    0 => Boolean::FALSE,
                    1 => Boolean::TRUE,
                    _ => unsafe { unreachable_unchecked() },
                },
            )
        })
    }
}

impl std::fmt::Debug for LiteralSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for literal in self.iter() {
            if first {
                first = false;
            } else {
                write!(f, ", ")?
            }
            write!(f, "{}", literal)?
        }
        write!(f, "}}")?;
        return Ok(());
    }
}
