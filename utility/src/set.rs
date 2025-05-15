use super::Map;

#[derive(Default, Clone)]
pub struct Set {
    map: Map<()>,
}

impl Set {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[inline(always)]
    pub fn contains_key(&self, index: usize) -> bool {
        self.map.contains_key(index)
    }

    pub fn is_subset_of(&self, set: impl Iterator<Item = usize>) -> bool {
        set.filter(|i| self.contains_key(*i)).count() == self.len()
    }

    #[inline(always)]
    pub fn insert(&mut self, index: usize) {
        self.map.insert(index, ());
    }

    #[inline(always)]
    pub fn extend(&mut self, indices: impl Iterator<Item = usize>) {
        self.map.extend(indices.map(|i| (i, ())));
    }

    #[inline(always)]
    pub fn remove(&mut self, index: usize) -> Option<()> {
        self.map.remove(index)
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<usize> {
        self.map.pop().map(|(index, _)| index)
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.map.clear();
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &usize> + Clone {
        self.map.iter().map(|(index, _)| index)
    }
}

impl std::fmt::Debug for Set {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}
