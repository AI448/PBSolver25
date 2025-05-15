pub struct Map<ValueT> {
    index_to_position: Vec<usize>,
    item_array: Vec<(usize, ValueT)>,
}

impl<ValueT> Default for Map<ValueT> {
    fn default() -> Self {
        Self {
            index_to_position: Vec::default(),
            item_array: Vec::default(),
        }
    }
}

impl<ValueT> Clone for Map<ValueT>
where
    ValueT: Clone,
{
    fn clone(&self) -> Self {
        Self {
            index_to_position: self.index_to_position.clone(),
            item_array: self.item_array.clone(),
        }
    }
}

impl<ValueT> Map<ValueT> {
    const NULL_POSITION: usize = usize::MAX;

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.item_array.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.item_array.is_empty()
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

    #[inline(always)]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ValueT> {
        if index >= self.index_to_position.len() {
            return None;
        } else {
            let position = self.index_to_position[index];
            if position == Self::NULL_POSITION {
                None
            } else {
                Some(&mut self.item_array[position].1)
            }
        }
    }

    #[inline(always)]
    pub fn insert(&mut self, index: usize, value: ValueT) {
        if index >= self.index_to_position.len() {
            self.index_to_position
                .resize(index + 1, Self::NULL_POSITION);
        }
        let position = &mut self.index_to_position[index];
        if *position == Self::NULL_POSITION {
            *position = self.item_array.len();
            self.item_array.push((index, value));
        } else {
            debug_assert!(self.item_array[*position].0 == index);
            self.item_array[*position].1 = value;
        }
    }

    pub fn extend(&mut self, items: impl Iterator<Item = (usize, ValueT)>) {
        for (index, value) in items {
            self.insert(index, value);
        }
    }

    #[inline(always)]
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
                };
                return Some(value);
            }
        }
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<(usize, ValueT)> {
        let option = self.item_array.pop();
        if let Some(item) = &option {
            self.index_to_position[item.0] = Self::NULL_POSITION;
        }
        return option;
    }

    pub fn retain(&mut self, mut f: impl FnMut(&usize, &mut ValueT) -> bool) {
        for k in (0..self.item_array.len()).rev() {
            let item = &mut self.item_array[k];
            if !f(&item.0, &mut item.1) {
                let removing_index = item.0;
                let moving_index = self.item_array.last().unwrap().0;
                debug_assert!(self.index_to_position[removing_index] == k);
                debug_assert!(self.index_to_position[moving_index] == self.item_array.len() - 1);
                self.index_to_position[moving_index] = k;
                self.index_to_position[removing_index] = Self::NULL_POSITION;
                self.item_array.swap_remove(k);
            }
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

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&usize, &mut ValueT)> {
        self.item_array
            .iter_mut()
            .map(|(index, value)| (&*index, value))
    }
}

impl<ValueT> std::fmt::Debug for Map<ValueT>
where
    ValueT: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}
