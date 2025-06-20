mod clique_constraint_theory;

use crate::{engine::EngineTrait, etc::State};


pub trait TheoryTrait {
    type ExplainKey: Copy;
    type ExplanationConstraint<'a> where Self: 'a;

    fn state(&self) -> State<Self::ExplainKey>;

    fn explain(&self, explain_key: Self::ExplainKey) -> Self::ExplanationConstraint<'_>;

    fn add_variable(&mut self);

    // TODO: EngineTrait ではなく DecisionStackTrait の方が良い？
    /// state().is_noconflict() であること
    fn propagate<EngineT>(&mut self, engine: &mut EngineT)
    where
        EngineT: EngineTrait,
        EngineT::CompositeExplainKey: From<Self::ExplainKey>;

    fn backjump<EngineT>(&mut self, backjump_level: usize, engine: &EngineT)
    where
        EngineT: EngineTrait;
}


pub trait TheoryAddConstraintTrait<ConstraintT>: TheoryTrait {
    fn add_constraint<EngineT>(&mut self, constraint: ConstraintT, is_learnt: bool, engine: &mut EngineT)
    where
        EngineT: EngineTrait,
        EngineT::CompositeExplainKey: From<Self::ExplainKey>;
}
