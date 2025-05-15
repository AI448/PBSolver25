use super::heap_sort;

pub struct HeapedMap<ValueT, CompareT>
where
    CompareT: Fn(&(usize, ValueT), &(usize, ValueT)) -> std::cmp::Ordering,
{
    compare: CompareT,
    index_to_position: Vec<usize>,
    item_array: Vec<(usize, ValueT)>,
}

impl<ValueT, CompareT> Default for HeapedMap<ValueT, CompareT>
where
    CompareT: Fn(&(usize, ValueT), &(usize, ValueT)) -> std::cmp::Ordering + Default,
{
    fn default() -> Self {
        Self {
            compare: CompareT::default(),
            index_to_position: Vec::default(),
            item_array: Vec::default(),
        }
    }
}

impl<ValueT, CompareT> Clone for HeapedMap<ValueT, CompareT>
where
    ValueT: Clone,
    CompareT: Fn(&(usize, ValueT), &(usize, ValueT)) -> std::cmp::Ordering + Clone,
{
    fn clone(&self) -> Self {
        Self {
            compare: self.compare.clone(),
            index_to_position: self.index_to_position.clone(),
            item_array: self.item_array.clone(),
        }
    }
}

impl<ValueT, CompareT> HeapedMap<ValueT, CompareT>
where
    CompareT: Fn(&(usize, ValueT), &(usize, ValueT)) -> std::cmp::Ordering,
{
    const NULL_POSITION: usize = usize::MAX;

    #[inline(always)]
    pub fn new(compare: CompareT) -> Self {
        Self {
            compare: compare,
            index_to_position: Vec::default(),
            item_array: Vec::default(),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.item_array.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.item_array.is_empty()
    }

    #[inline(always)]
    pub fn first(&self) -> Option<(&usize, &ValueT)> {
        self.item_array.first().map(|(index, value)| (index, value))
    }

    #[inline(always)]
    pub fn contains_key(&self, index: usize) -> bool {
        if index >= self.index_to_position.len() {
            return false;
        } else {
            return self.index_to_position[index] != Self::NULL_POSITION;
        }
    }

    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&ValueT> {
        if index >= self.index_to_position.len() {
            return None;
        } else {
            let position = self.index_to_position[index];
            if position == Self::NULL_POSITION {
                None
            } else {
                Some(&self.item_array[position].1)
            }
        }
    }

    pub fn insert(&mut self, index: usize, value: ValueT) {
        if index >= self.index_to_position.len() {
            self.index_to_position
                .resize(index + 1, Self::NULL_POSITION);
        }
        let position = &mut self.index_to_position[index];
        if *position == Self::NULL_POSITION {
            *position = self.item_array.len();
            self.item_array.push((index, value));
            let position = *position;
            heap_sort::up_heap_with_callback(
                &mut self.item_array,
                position,
                &self.compare,
                |a, b| self.index_to_position.swap(a.0, b.0),
            );
        } else {
            debug_assert!(self.item_array[*position].0 == index);
            self.item_array[*position].1 = value;
            let position = *position;
            heap_sort::update_heap_with_callback(
                &mut self.item_array,
                position,
                &self.compare,
                |a, b| self.index_to_position.swap(a.0, b.0),
            );
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<ValueT> {
        if index >= self.index_to_position.len() {
            return None;
        } else {
            let position = self.index_to_position[index];
            if position == Self::NULL_POSITION {
                return None;
            } else {
                debug_assert!(self.item_array[position].0 == index);
                let value = self.item_array.swap_remove(position).1;
                self.index_to_position[index] = Self::NULL_POSITION;
                if position != self.item_array.len() {
                    debug_assert!(
                        self.index_to_position[self.item_array[position].0]
                            == self.item_array.len()
                    );
                    self.index_to_position[self.item_array[position].0] = position;
                    heap_sort::update_heap_with_callback(
                        &mut self.item_array,
                        position,
                        &self.compare,
                        |a, b| self.index_to_position.swap(a.0, b.0),
                    );
                };
                return Some(value);
            }
        }
    }

    pub fn pop_first(&mut self) -> Option<(usize, ValueT)> {
        if self.item_array.is_empty() {
            return None;
        } else {
            let (index, value) = self.item_array.swap_remove(0);
            debug_assert!(self.index_to_position[index] == 0);
            self.index_to_position[index] = Self::NULL_POSITION;
            if !self.item_array.is_empty() {
                debug_assert!(
                    self.index_to_position[self.item_array[0].0] == self.item_array.len()
                );
                self.index_to_position[self.item_array[0].0] = 0;
                heap_sort::down_heap_with_callback(
                    &mut self.item_array,
                    0,
                    &self.compare,
                    |a, b| self.index_to_position.swap(a.0, b.0),
                );
            };
            return Some((index, value));
        }
    }

    pub fn clear(&mut self) {
        while !self.item_array.is_empty() {
            let index = unsafe { self.item_array.pop().unwrap_unchecked() }.0;
            debug_assert!(self.index_to_position[index] == self.item_array.len());
            self.index_to_position[index] = Self::NULL_POSITION;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&usize, &ValueT)> + Clone {
        self.item_array.iter().map(|(index, value)| (index, value))
    }
}

impl<ValueT, CompareT> std::fmt::Debug for HeapedMap<ValueT, CompareT>
where
    ValueT: std::fmt::Debug,
    CompareT: Fn(&(usize, ValueT), &(usize, ValueT)) -> std::cmp::Ordering,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}
