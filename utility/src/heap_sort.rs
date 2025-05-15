// use std::cmp::Ordering::Less;

#[inline(always)]
fn parent_of(position: usize) -> usize {
    debug_assert!(position != 0);
    return (position + 1) / 2 - 1;
}

#[inline(always)]
fn left_of(position: usize) -> usize {
    return (position + 1) * 2 - 1;
}

#[inline(always)]
fn right_of(position: usize) -> usize {
    (position + 1) * 2
}

#[inline(always)]
pub fn up_heap<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    compare: impl Fn(&ValueT, &ValueT) -> std::cmp::Ordering,
) {
    up_heap_with_callback(array, position, compare, |_, _| ());
}

#[inline(always)]
pub fn down_heap<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    compare: impl std::ops::Fn(&ValueT, &ValueT) -> std::cmp::Ordering,
) {
    down_heap_with_callback(array, position, compare, |_, _| ());
}

#[inline(always)]
pub fn update_heap<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    compare: impl std::ops::Fn(&ValueT, &ValueT) -> std::cmp::Ordering,
) {
    update_heap_with_callback(array, position, compare, |_, _| ());
}

#[inline(always)]
pub fn update_heap_with_callback<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    compare: impl Fn(&ValueT, &ValueT) -> std::cmp::Ordering,
    callback_swap: impl FnMut(&ValueT, &ValueT),
) {
    debug_assert!(position < array.len());
    if position != 0
        && compare(&array[position], &array[parent_of(position)]) == std::cmp::Ordering::Less
    {
        up_heap_with_callback(array, position, compare, callback_swap);
    } else {
        down_heap_with_callback(array, position, compare, callback_swap);
    }
}

pub fn up_heap_with_callback<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    compare: impl Fn(&ValueT, &ValueT) -> std::cmp::Ordering,
    mut callback_swap: impl FnMut(&ValueT, &ValueT),
) {
    debug_assert!(position < array.len());
    let mut current = position;
    loop {
        if current == 0 {
            break;
        }
        let parent = parent_of(current);
        if compare(&array[current], &array[parent]) == std::cmp::Ordering::Less {
            array.swap(parent, current);
            callback_swap(&array[parent], &array[current]);
            current = parent;
        } else {
            break;
        }
    }
}

pub fn down_heap_with_callback<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    compare: impl std::ops::Fn(&ValueT, &ValueT) -> std::cmp::Ordering,
    mut callback_swap: impl FnMut(&ValueT, &ValueT),
) {
    debug_assert!(position < array.len());
    let mut current = position;
    loop {
        let left = left_of(current);
        if left >= array.len() {
            break;
        }
        let right = right_of(current);
        let child = if right >= array.len()
            || compare(&array[left], &array[right]) == std::cmp::Ordering::Less
        {
            left
        } else {
            right
        };
        if compare(&array[child], &array[current]) == std::cmp::Ordering::Less {
            array.swap(current, child);
            callback_swap(&array[child], &array[current]);
            current = child;
        } else {
            break;
        }
    }
}
