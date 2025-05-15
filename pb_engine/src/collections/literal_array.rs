use std::ops::{Index, IndexMut};

use crate::types::Literal;

#[derive(Default, Clone, Debug)]
pub struct LiteralArray<ValueT> {
    array: Vec<[ValueT; 2]>,
}

impl<ValueT> LiteralArray<ValueT> {
    #[must_use]
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.array.len()
    }

    #[inline(always)]
    pub fn push(&mut self, value: [ValueT; 2]) {
        self.array.push(value);
    }

    // pub fn resize_with(&mut self, new_len: usize, f: impl Fn() -> [ValueT; 2]) {
    //     if self.array.len() < new_len * 2 {
    //         while self.array.len() < new_len * 2 {
    //             self.push(f());
    //         }
    //     } else if self.array.len() > new_len * 2 {
    //         self.array.truncate(new_len * 2);
    //     }
    // }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut [ValueT; 2]> + '_ {
        self.array.iter_mut()
    }
}

impl<ValueT> Index<Literal> for LiteralArray<ValueT> {
    type Output = ValueT;
    #[inline(always)]
    fn index(&self, literal: Literal) -> &Self::Output {
        &self.array[literal.index()][literal.value()]
    }
}

impl<ValueT> IndexMut<Literal> for LiteralArray<ValueT> {
    #[inline(always)]
    fn index_mut(&mut self, literal: Literal) -> &mut Self::Output {
        &mut self.array[literal.index()][literal.value()]
    }
}
