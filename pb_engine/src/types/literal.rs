use std::{hint::unreachable_unchecked, ops::Not};

use super::boolean::Boolean;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Literal {
    bits: usize,
}

impl Literal {
    #[inline(always)]
    pub fn new(index: usize, value: Boolean) -> Self {
        debug_assert!(((index << 1) >> 1) == index);
        return Self {
            bits: (index << 1) | value as usize,
        };
    }

    #[inline(always)]
    pub fn index(&self) -> usize {
        return self.bits >> 1;
    }

    #[inline(always)]
    pub fn value(&self) -> Boolean {
        return match self.bits & 1 {
            0 => Boolean::FALSE,
            1 => Boolean::TRUE,
            _ => {
                debug_assert!(false);
                unsafe { unreachable_unchecked() }
            }
        };
    }

    #[inline(always)]
    pub fn bits(&self) -> usize {
        self.bits
    }
}

impl Not for Literal {
    type Output = Literal;
    #[inline(always)]
    fn not(self) -> Self::Output {
        Literal {
            bits: self.bits ^ 1,
        }
    }
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}l_{}",
            if self.value() == Boolean::FALSE {
                "!"
            } else {
                ""
            },
            self.index()
        )
    }
}

impl std::fmt::Debug for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Display>::fmt(self, f)
    }
}
