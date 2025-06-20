use std::{cmp::min, fmt::Debug};

use either::Either;

// #[derive(Clone, Copy, PartialEq, Eq, Debug)]
// pub enum CompositeExplainKey {
//     MonadicClause(MonadicConstraintExplainKey),
//     // CliqueConstraint(CliqueExplainKey),
//     // CountConstraint(CountConstraintExplainKey),
//     // IntegerLinearConstraint(IntegerLinearConstraintExplainKey),
// }

// impl From<MonadicConstraintExplainKey> for CompositeExplainKey {
//     fn from(explain_key: MonadicConstraintExplainKey) -> Self {
//         return Self::MonadicClause(explain_key);
//     }
// }

// impl From<CliqueExplainKey> for CompositeExplainKey {
//     fn from(explain_key: CliqueExplainKey) -> Self {
//         return Self::CliqueConstraint(explain_key);
//     }
// }

// #[derive(Clone, Debug)]
// pub enum CompositeConstraint<CliqueConstraintT>
// where
//     CliqueConstraintT: CliqueConstraintTrait
// {
//     MonadicClause(MonadicClause),
//     CliqueConstraint(CliqueConstraintT)
// }

#[derive(Clone, Copy, Debug)]
pub enum State<ExplainKeyT> {
    /// 現在の決定レベルよりも前に伝播が発生する制約条件が追加された
    BackjumpRequired { backjump_level: usize },
    /// 現在の決定レベルで矛盾が発生している
    Conflict { explain_key: ExplainKeyT },
    /// それ以外
    Noconflict,
}

impl<ExplainKeyT> State<ExplainKeyT> {
    pub fn is_backjump_required(&self) -> bool {
        return matches!(self, Self::BackjumpRequired { .. });
    }

    pub fn is_backjump_required_and(&self, f: impl Fn(usize) -> bool) -> bool {
        return match self {
            Self::BackjumpRequired { backjump_level } => f(*backjump_level),
            _ => false,
        };
    }

    pub fn is_conflict(&self) -> bool {
        return matches!(self, Self::Conflict { .. });
    }

    pub fn is_noconflict(&self) -> bool {
        return matches!(self, Self::Noconflict);
    }

    pub fn is_prior_to<OtherExplainKeyT>(&self, other: &State<OtherExplainKeyT>) -> bool {
        fn to_tuple<T>(state: &State<T>) -> (usize, usize) {
            return match state {
                State::BackjumpRequired { backjump_level } => (0, *backjump_level),
                State::Conflict { .. } => (1, 0),
                State::Noconflict => (2, 0),
            };
        }

        return to_tuple(self) < to_tuple(&other);
    }

    pub fn merge(&mut self, other: State<ExplainKeyT>) {
        if other.is_prior_to(self) {
            *self = other;
        }
    }

    pub fn composite<OtherExplainKeyT>(
        &self,
        other: State<OtherExplainKeyT>,
    ) -> State<Either<ExplainKeyT, OtherExplainKeyT>>
    where
        ExplainKeyT: Copy,
        OtherExplainKeyT: Copy,
    {
        if self.is_prior_to(&other) {
            return match self {
                State::BackjumpRequired { backjump_level } => State::BackjumpRequired {
                    backjump_level: *backjump_level,
                },
                State::Conflict { explain_key } => State::Conflict {
                    explain_key: Either::Left(*explain_key),
                },
                State::Noconflict => State::Noconflict,
            };
        } else {
            return match other {
                State::BackjumpRequired { backjump_level } => {
                    State::BackjumpRequired { backjump_level }
                }
                State::Conflict { explain_key } => State::Conflict {
                    explain_key: Either::Right(explain_key),
                },
                State::Noconflict => State::Noconflict,
            };
        }
    }
}

/// 割り当て理由
#[derive(Clone, Copy, Debug)]
pub enum Reason<ExplainKeyT> {
    /// 決定
    Decision,
    /// 伝播
    Propagation {
        // NOTE: 伝播がどの時点で発生したかをここに持たせることも考えられるが， Conflict Analisis で特定したほうが効率的
        /// 伝播を引き起こした制約条件
        explain_key: ExplainKeyT,
    },
}

impl<ExplainKeyT> Reason<ExplainKeyT> {
    #[inline(always)]
    pub fn is_decision(&self) -> bool {
        return matches!(self, Self::Decision);
    }
    #[inline(always)]
    pub fn is_propagation(&self) -> bool {
        return matches!(self, Self::Propagation { .. });
    }
}
