use std::ops::{Index, IndexMut, Not};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Boolean {
    FALSE = 0,
    TRUE = 1,
}

impl Not for Boolean {
    type Output = Boolean;
    #[inline(always)]
    fn not(self) -> Self::Output {
        match self {
            Self::FALSE => Self::TRUE,
            Self::TRUE => Self::FALSE,
        }
    }
}

impl From<bool> for Boolean {
    #[inline(always)]
    fn from(value: bool) -> Self {
        match value {
            false => Self::FALSE,
            true => Self::TRUE,
        }
    }
}

// impl From<u8> for Boolean {
//     fn from(value: u8) -> Self {
//         debug_assert!(value <= 1);
//         match value {
//             0 => Self::FALSE,
//             1 => Self::TRUE,
//             _ => unsafe { unreachable_unchecked() },
//         }
//     }
// }

// impl From<u32> for Boolean {
//     fn from(value: u32) -> Self {
//         debug_assert!(value <= 1);
//         match value {
//             0 => Self::FALSE,
//             1 => Self::TRUE,
//             _ => unsafe { unreachable_unchecked() },
//         }
//     }
// }

impl<T> Index<Boolean> for [T; 2] {
    type Output = T;
    #[inline(always)]
    fn index(&self, index: Boolean) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<Boolean> for [T; 2] {
    #[inline(always)]
    fn index_mut(&mut self, index: Boolean) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

impl std::fmt::Display for Boolean {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", if *self == Boolean::FALSE { "F" } else { "T" })
    }
}
