use crate::Literal;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MonadicClause {
    pub literal: Literal,
}
