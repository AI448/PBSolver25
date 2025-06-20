use crate::Literal;

pub trait CountConstraintTrait {
    fn iter_terms(&self) -> impl Iterator<Item = Literal> + Clone + '_;
    fn lower(&self) -> u64;
    fn len(&self) -> usize {
        self.iter_terms().count()
    }
}

impl<CountConstraintT> CountConstraintTrait for &CountConstraintT
where
    CountConstraintT: CountConstraintTrait,
{
    fn iter_terms(&self) -> impl Iterator<Item = Literal> + Clone + '_ {
        (*self).iter_terms()
    }
    fn lower(&self) -> u64 {
        (*self).lower()
    }
    fn len(&self) -> usize {
        (*self).len()
    }
}

#[derive(Clone, Debug)]
pub struct CountConstraint {
    literals: Vec<Literal>,
    lower: u64,
}

impl CountConstraintTrait for CountConstraint {
    fn iter_terms(&self) -> impl Iterator<Item = Literal> + Clone + '_ {
        self.literals.iter().cloned()
    }

    fn lower(&self) -> u64 {
        self.lower
    }
}

#[derive(Clone, Debug)]
pub struct CountConstraintView<IteratorT: Iterator<Item = Literal> + Clone> {
    literals: IteratorT,
    lower: u64,
}

impl<IteratorT> CountConstraintView<IteratorT>
where
    IteratorT: Iterator<Item = Literal> + Clone,
{
    pub fn new(literals: IteratorT, lower: u64) -> Self {
        Self { literals, lower }
    }
}

impl<IteratorT: Iterator<Item = Literal> + Clone> CountConstraintTrait
    for CountConstraintView<IteratorT>
{
    fn iter_terms(&self) -> impl Iterator<Item = Literal> + Clone + '_ {
        self.literals.clone()
    }

    fn lower(&self) -> u64 {
        self.lower
    }
}


