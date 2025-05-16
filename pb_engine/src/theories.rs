use crate::{decision_stack::DecisionStack, types::Literal};

mod count_constraint_theory;
mod integer_linear_constraint_theory;
mod monadic_clause_theory;

// TODO: 名前を検討
pub struct Propagation<ExplainKey: Copy> {
    pub literal: Literal,
    pub explain_key: ExplainKey,
    pub plbd: usize,
}

pub trait TheoryTrait {
    type ExplainKey: Copy;
    type ExplanationConstraint<'a>
    where
        Self: 'a;

    fn add_variable(&mut self);

    fn assign<ExplainKeyT: Copy>(
        &mut self,
        decision_stack: &DecisionStack<ExplainKeyT>,
        callback: impl FnMut(Propagation<Self::ExplainKey>),
    );

    fn backjump<ExplainKeyT: Copy>(
        &mut self,
        backjump_level: usize,
        decision_stack: &DecisionStack<ExplainKeyT>,
    );

    fn explain(&self, explain_key: Self::ExplainKey) -> Self::ExplanationConstraint<'_>;
}

pub trait TheoryAddConstraintTrait<ConstraintT>: TheoryTrait {
    fn add_constraint<ExplainKeyT: Copy>(
        &mut self,
        constraint: ConstraintT,
        decision_stack: &DecisionStack<ExplainKeyT>,
        callback: impl FnMut(Propagation<Self::ExplainKey>),
    ) -> Result<(), usize>;

    // TODO: 制約条件を追加するための適切な決定レベルを算出する関数を追加
}

pub use count_constraint_theory::{CountConstraintExplainKey, CountConstraintTheory};
pub use integer_linear_constraint_theory::{
    IntegerLinearConstraintExplainKey, IntegerLinearConstraintTheory,
};
pub use monadic_clause_theory::{MonadicClauseExplainKey, MonadicClauseTheory};
