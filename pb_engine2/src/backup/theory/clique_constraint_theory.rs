use std::collections::VecDeque;
use crate::{collections::LiteralArray, engine::EngineTrait, etc::{Reason, State}, theory::{TheoryAddConstraintTrait, TheoryTrait}, Literal};


pub trait CliqueConstraintTrait {
    fn iter_literals(&self) -> impl Iterator<Item = Literal>;
}

#[derive(Clone, Debug)]
pub struct CliqueConstraint {
    literals: Vec<Literal>,
}

impl CliqueConstraint {
    pub fn new(literals: impl Iterator<Item = Literal>) -> Self {
        return Self {
            literals: literals.collect(),
        };
    }
}

impl CliqueConstraintTrait for CliqueConstraint {
    fn iter_literals(&self) -> impl Iterator<Item = Literal> {
        self.literals.iter().cloned()
    }
}

#[derive(Clone, Debug)]
pub struct CliqueConstraintView<IteratorT> {
    iterator: IteratorT,
}

impl<IteratorT> CliqueConstraintView<IteratorT> {
    pub fn new(iterator: IteratorT) -> Self {
        Self { iterator }
    }
}

impl<IteratorT> CliqueConstraintTrait for CliqueConstraintView<IteratorT>
where
    IteratorT: Iterator<Item = Literal> + Clone
{
    fn iter_literals(&self) -> impl Iterator<Item = Literal> + Clone {
        self.iterator.clone()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CliqueConstraintTheoryExplainKey {
    row_id: usize
}


#[derive(Clone, Debug)]
struct Row {
    literals: Vec<Literal>,
}

#[derive(Default, Clone, Debug)]
struct Column {
    row_ids: Vec<usize>,
}

pub struct CliqueConstraintTheory {
    state: State<CliqueConstraintTheoryExplainKey>,
    rows: Vec<Row>,
    columns: LiteralArray<Column>,
    number_of_evaluated_assignments: usize,
    constraint_queue: VecDeque<(CliqueConstraint, bool)>,
}

impl CliqueConstraintTheory {

    pub fn new() -> Self {
        Self {
            state: State::Noconflict,
            rows: Vec::default(),
            columns: LiteralArray::default(),
            number_of_evaluated_assignments: 0,
            constraint_queue: VecDeque::default(),
        }
    }

    fn propagate_by_assignment<EngineT>(&mut self, engine: &mut EngineT)
    where
        EngineT: EngineTrait,
        EngineT::CompositeExplainKey: From<CliqueConstraintTheoryExplainKey>
    {
        debug_assert!(engine.state().is_noconflict());
        debug_assert!(self.state.is_noconflict());
        debug_assert!(self.number_of_evaluated_assignments < engine.number_of_assignments());

        let assignment = engine.get_assignment(self.number_of_evaluated_assignments);
        self.number_of_evaluated_assignments += 1;

        for &row_id in self.columns[!assignment].row_ids.iter() {
            let row = &self.rows[row_id];
            debug_assert!(row.literals.iter().find(|&&literal| literal == !assignment).is_some());
            let explain_key = CliqueConstraintTheoryExplainKey { row_id };
            for &literal in row.literals.iter() {
                if literal == !assignment {
                    continue;
                }
                if !engine.is_assigned(literal.index()) {
                    // 伝播
                    engine.assign(literal, Reason::Propagation { explain_key: explain_key.into() });
                    if !engine.state().is_noconflict() {
                        return;
                    }
                } else if engine.is_false(literal) {
                    // 矛盾
                    // 現在の決定レベルより前に False が割り当てられていることはないはず
                    debug_assert!(engine.get_decision_level(literal.index()) == engine.decision_level());
                    self.state = State::Conflict { explain_key };
                    return;
                }
            }
        }
    }

    fn propagate_by_constraint_addition<EngineT>(&mut self, engine: &mut EngineT)
    where
        EngineT: EngineTrait,
        EngineT::CompositeExplainKey: From<CliqueConstraintTheoryExplainKey>
    {
        debug_assert!(engine.state().is_noconflict());
        debug_assert!(self.state.is_noconflict());
        debug_assert!(self.number_of_evaluated_assignments == engine.number_of_assignments()); // いる？
        debug_assert!(!self.constraint_queue.is_empty());

        let (constraint, is_learnt) = self.constraint_queue.pop_front().unwrap();
        debug_assert!(
            !constraint
                .iter_literals()
                .any(|literal| engine.is_false(literal) && engine.get_decision_level(literal.index()) < engine.decision_level())
        );

        let row_id = self.rows.len();
        self.rows.push(Row {
            literals: constraint.literals,
        });
        let row = self.rows.last().unwrap();

        for &literal in row.literals.iter() {
            self.columns[literal].row_ids.push(row_id);
        }

        let explain_key = CliqueConstraintTheoryExplainKey { row_id };

        let number_of_falsified_literals = row
            .literals
            .iter()
            .filter(|&&literal| engine.is_false(literal))
            .count();
        if number_of_falsified_literals == 1 {
            let &falsified_literal = row
                .literals
                .iter()
                .find(|&&literal| engine.is_false(literal))
                .unwrap();
            for &literal in row.literals.iter() {
                if literal == falsified_literal {
                    continue;
                }
                if !engine.is_assigned(literal.index()) {
                    engine.assign(
                        literal,
                        Reason::Propagation {
                            explain_key: explain_key.into(),
                        },
                    );
                    if !engine.state().is_noconflict() {
                        return;
                    }
                } else if engine.is_false(literal) {
                    self.state = State::Conflict { explain_key };
                    return;
                }
            }
        } else if number_of_falsified_literals >= 2 {
            self.state = State::Conflict { explain_key };
            return;
        }
    }

}

impl TheoryTrait for CliqueConstraintTheory {
    type ExplainKey = CliqueConstraintTheoryExplainKey;
    type ExplanationConstraint<'a> = impl CliqueConstraintTrait;

    fn state(&self) -> State<Self::ExplainKey> {
        return self.state;
    }

    fn explain(&self, explain_key: Self::ExplainKey) -> Self::ExplanationConstraint<'_> {
        let row = &self.rows[explain_key.row_id];
        return CliqueConstraintView::new(row.literals.iter().cloned());
    }

    fn add_variable(&mut self) {
        self.columns.push([Column::default(), Column::default()]);
    }

    fn propagate<EngineT>(&mut self, engine: &mut EngineT)
    where
        EngineT: EngineTrait,
        EngineT::CompositeExplainKey: From<Self::ExplainKey>
    {
        loop {
            while self.number_of_evaluated_assignments < engine.number_of_assignments() {
                if !engine.state().is_noconflict() || !self.state.is_noconflict() {
                    return;
                }
                self.propagate_by_assignment(engine);
            }
            if !engine.state().is_noconflict()
                || !self.state.is_noconflict()
                || self.constraint_queue.is_empty()
            {
                return;
            }
            self.propagate_by_constraint_addition(engine);
        }
    }

    fn backjump<EngineT>(&mut self, backjump_level: usize, engine: &EngineT)
    where
        EngineT: EngineTrait
    {
        let backjump_order = engine.order_range(backjump_level).end;
        debug_assert!(backjump_order <= self.number_of_evaluated_assignments);
        self.number_of_evaluated_assignments = backjump_order;
        // if let State::BackjumpRequired {backjump_level: required_backjump_level} = self.state
        //     && backjump_level <= required_backjump_level
        // {
        //     self.state = State::Noconflict;
        // }
    }
}


impl<ConstraintT> TheoryAddConstraintTrait<ConstraintT> for CliqueConstraintTheory
where
    ConstraintT: CliqueConstraintTrait
{
    fn add_constraint<EngineT>(&mut self, constraint: ConstraintT, is_learnt: bool, engine: &mut EngineT)
    where
        EngineT: EngineTrait,
        EngineT::CompositeExplainKey: From<Self::ExplainKey>,
    {
        self.constraint_queue.push_back((
            CliqueConstraint::new(constraint.iter_literals()),
            is_learnt
        ));

        let min_falsified_decision_level = self.constraint_queue.back().unwrap().0
            .iter_literals()
            .filter(|&literal| engine.is_false(literal))
            .map(|literal| engine.get_decision_level(literal.index()))
            .min();

        if let Some(min_falsified_decision_level) = min_falsified_decision_level && min_falsified_decision_level < engine.decision_level() {
            self.state = State::BackjumpRequired { backjump_level: min_falsified_decision_level };
        } else {
            self.propagate(engine);
        }
    }
}
