use std::collections::HashSet;
use std::hash::Hash;

pub trait IteratorExt {
    type Item;

    fn collect_set(self) -> HashSet<Self::Item>;
}

impl<T> IteratorExt for T
where
    T: Iterator,
    T::Item: Eq + Hash + Clone,
{
    type Item = T::Item;
    fn collect_set(self) -> HashSet<Self::Item> {
        self.collect()
    }
}
