use std::collections::{HashMap, HashSet};
use std::hash::Hash;

pub trait Difference<'a> {
    type Diff;

    fn difference(&'a self, other: &'a Self) -> Self::Diff;
}

pub enum ValueDifference<T> {
    LeftMissing(T),
    RightMissing(T),
    Different(T, T),
}

impl<'a, K, V> Difference<'a> for HashMap<K, V>
where
    K: Hash + Eq + 'a,
    V: Eq + 'a,
{
    type Diff = HashMap<&'a K, ValueDifference<&'a V>>;

    fn difference(&'a self, other: &'a Self) -> Self::Diff {
        let mut result = HashMap::new();

        let mut keys = self.keys().collect::<HashSet<_>>();
        keys.extend(other.keys());

        for key in keys {
            match (self.get(key), other.get(key)) {
                (Some(v), Some(other_v)) if v != other_v => {
                    result.insert(key, ValueDifference::Different(v, other_v));
                }
                (Some(v), None) => {
                    result.insert(key, ValueDifference::RightMissing(v));
                }
                (None, Some(other_v)) => {
                    result.insert(key, ValueDifference::LeftMissing(other_v));
                }
                _ => {}
            }
        }

        result
    }
}
