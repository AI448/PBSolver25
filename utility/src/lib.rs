#![feature(unboxed_closures)]
#![feature(fn_traits)]

mod calculate_gcd;
mod heap_sort;
mod heaped_map;
mod map;
mod priority_queue;
mod set;

pub use heap_sort::{
    down_heap, down_heap_with_callback, up_heap, up_heap_with_callback, update_heap,
    update_heap_with_callback,
};
pub use heaped_map::HeapedMap;
pub use map::Map;
pub use priority_queue::PriorityQueue;
pub use set::Set;

// pub trait Fmax {
//     type Output;
//     fn fmax(self) -> Self::Output;
//     fn fmin(self) -> Self::Output;
// }

// impl<IteratorT, FloatT> Fmax for IteratorT
// where
//     IteratorT: Iterator<Item = FloatT>,
//     FloatT: num::Float,
// {
//     type Output = FloatT;
//     fn fmax(self) -> Self::Output {
//         let mut max = FloatT::neg_infinity();
//         for x in self {
//             if x.is_nan() || x == FloatT::infinity() {
//                 return x;
//             }
//             if x > max {
//                 max = x;
//             }
//         }
//         return max;
//     }

//     fn fmin(self) -> Self::Output {
//         let mut min = FloatT::infinity();
//         for x in self {
//             if x.is_nan() || x == FloatT::neg_infinity() {
//                 return x;
//             }
//             if x < min {
//                 min = x;
//             }
//         }
//         return min;
//     }
// }
