#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(impl_trait_in_assoc_type)]

mod analyze;
mod calculate_plbd;
mod collections;
mod constraints;
mod decision_stack;
mod engine;
mod theories;
mod types;

pub use analyze::{Analyze, AnalyzeResult, StrengthenLinearConstraint};
pub use calculate_plbd::CalculatePLBD;
pub use constraints::{
    CountConstraint, CountConstraintTrait, CountConstraintView, LinearConstraint,
    LinearConstraintTrait, LinearConstraintView, MonadicClause,
};
pub use engine::{PBConstraint, PBEngine, PBExplainKey, PBState, Reason};
pub use types::{Boolean, Literal};
