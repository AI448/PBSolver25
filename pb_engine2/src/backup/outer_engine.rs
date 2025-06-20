use std::ops::Deref;

use either::Either;

use crate::{engine::{EngineAddConstraintTrait, EngineTrait}, etc::{Reason, State}, theory::{TheoryAddConstraintTrait, TheoryTrait}, Literal};


pub struct OuterEngine<TheoryT, InnerEngineT> {
    theory: TheoryT,
    inner_engine: InnerEngineT,
}

impl<TheoryT, InnerEngineT> OuterEngine<TheoryT, InnerEngineT> {
    pub fn new(theory: TheoryT, inner_engine: InnerEngineT) -> Self {
        OuterEngine { theory, inner_engine }
    }
}


impl<TheoryT, InnerEngineT> Deref for OuterEngine<TheoryT, InnerEngineT>
where
    InnerEngineT: Deref
{
    type Target = InnerEngineT::Target;
    fn deref(&self) -> &Self::Target {
        self.inner_engine.deref()
    }
}

impl<TheoryT, InnerEngineT> EngineTrait for OuterEngine<TheoryT, InnerEngineT>
where
    TheoryT: TheoryTrait,
    InnerEngineT: EngineTrait,
    InnerEngineT::CompositeExplainKey: From<TheoryT::ExplainKey> + TryInto<TheoryT::ExplainKey>
{
    type CompositeExplainKey = InnerEngineT::CompositeExplainKey;
    type ExplainKey = Either<TheoryT::ExplainKey, InnerEngineT::ExplainKey>;
    type ExplanationConstraint<'a> = Either<TheoryT::ExplanationConstraint<'a>, InnerEngineT::ExplanationConstraint<'a>> where Self: 'a;

    fn state(&self) -> State<Self::ExplainKey> {
        return self.theory.state().merge(self.inner_engine.state());
    }

    fn explain(&self, explain_key: Self::ExplainKey) -> Self::ExplanationConstraint<'_> {
        return match explain_key {
            Either::Left(explain_key) => {
                Either::Left(self.theory.explain(explain_key))
            },
            Either::Right(explain_key) => {
                Either::Right(self.inner_engine.explain(explain_key))
            }
        };
    }

    fn add_variable(&mut self) {
        self.inner_engine.add_variable();
        self.theory.add_variable();
    }

    fn assign(&mut self, literal: Literal, reason: Reason<Self::CompositeExplainKey>) {
        if let Reason::Propagation { explain_key } = reason {
            assert!(explain_key.try_into().is_err());
        }
        self.inner_engine.assign(literal, reason);
        if self.inner_engine.state().is_noconflict() {
            self.theory.propagate(&mut self.inner_engine);
        }
    }

    fn backjump(&mut self, backjump_level: usize) {
        self.theory.backjump(backjump_level, &self.inner_engine);
        self.inner_engine.backjump(backjump_level);
        if self.inner_engine.state().is_noconflict() {
            self.theory.propagate(&mut self.inner_engine);
        }
    }

}


impl<TheoryT, InnerEngineT, TheoryConstraintT, InnerConstraintT> EngineAddConstraintTrait<Either<TheoryConstraintT, InnerConstraintT>> for OuterEngine<TheoryT, InnerEngineT>
where
    TheoryT: TheoryTrait + TheoryAddConstraintTrait<TheoryConstraintT>,
    InnerEngineT: EngineTrait + EngineAddConstraintTrait<InnerConstraintT>,
    InnerEngineT::CompositeExplainKey: From<TheoryT::ExplainKey> + TryInto<TheoryT::ExplainKey>
{
    fn add_constraint(&mut self, constraint: Either<TheoryConstraintT, InnerConstraintT>, is_learnt: bool) {
        match constraint {
            Either::Left(constraint) => {
                self.theory.add_constraint(constraint, is_learnt, &mut self.inner_engine);
            },
            Either::Right(constraint) => {
                self.inner_engine.add_constraint(constraint, is_learnt);
                self.theory.propagate(&mut self.inner_engine);
            }
        }
        
    }
}