use super::heap_sort;

pub struct PriorityQueue<ValueT, CompareT>
where
    CompareT: Fn(&ValueT, &ValueT) -> std::cmp::Ordering,
{
    compare: CompareT,
    array: Vec<ValueT>,
}

impl<ValueT, CompareT> PriorityQueue<ValueT, CompareT>
where
    CompareT: Fn(&ValueT, &ValueT) -> std::cmp::Ordering,
{
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.array.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.array.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ValueT> + Clone + '_ {
        self.array.iter()
    }

    #[inline(always)]
    pub fn peek(&self) -> Option<&ValueT> {
        self.array.first()
    }

    // pub fn reserve(&mut self, additional: usize) {
    //     self.array.reserve(additional);
    // }

    pub fn push(&mut self, value: ValueT) {
        let position = self.array.len();
        self.array.push(value);
        heap_sort::up_heap(&mut self.array, position, &self.compare);
    }

    pub fn pop(&mut self) -> Option<ValueT> {
        if self.array.is_empty() {
            return None;
        } else {
            let value = self.array.swap_remove(0);
            if !self.array.is_empty() {
                heap_sort::down_heap(&mut self.array, 0, &self.compare);
            }
            return Some(value);
        }
    }

    pub fn clear(&mut self) {
        self.array.clear();
    }
}

impl<ValueT, CompareT> Default for PriorityQueue<ValueT, CompareT>
where
    CompareT: Fn(&ValueT, &ValueT) -> std::cmp::Ordering + Default,
{
    fn default() -> Self {
        Self {
            compare: CompareT::default(),
            array: Vec::default(),
        }
    }
}

impl<ValueT, CompareT> Clone for PriorityQueue<ValueT, CompareT>
where
    ValueT: Clone,
    CompareT: Fn(&ValueT, &ValueT) -> std::cmp::Ordering + Clone,
{
    fn clone(&self) -> Self {
        Self {
            compare: self.compare.clone(),
            array: self.array.clone(),
        }
    }
}

impl<ValueT, CompareT> std::fmt::Debug for PriorityQueue<ValueT, CompareT>
where
    CompareT: Fn(&ValueT, &ValueT) -> std::cmp::Ordering + Clone,
    Vec<ValueT>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.array.fmt(f)
    }
}
