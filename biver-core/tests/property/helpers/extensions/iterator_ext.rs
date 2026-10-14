use std::collections::HashSet;
use std::hash::Hash;

pub trait IteratorExt {
    type Item;

    fn _except(self, item: &Self::Item) -> impl Iterator<Item = Self::Item>;

    fn except_some(self, item: &Option<Self::Item>) -> impl Iterator<Item = Self::Item>;

    fn collect_set(self) -> HashSet<Self::Item>;
}

impl<T> IteratorExt for T
where
    T: Iterator,
    T::Item: Eq + Hash + Clone,
{
    type Item = T::Item;

    fn _except(self, item: &Self::Item) -> impl Iterator<Item = Self::Item> {
        self.filter(move |i| i != item)
    }

    fn except_some(self, item: &Option<Self::Item>) -> impl Iterator<Item = Self::Item> {
        self.filter(move |i| Some(i) != item.as_ref())
    }

    fn collect_set(self) -> HashSet<Self::Item> {
        self.collect()
    }
}
