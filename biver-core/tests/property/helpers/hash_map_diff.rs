use crate::helpers::diff::Diff;
use crate::helpers::extensions::IteratorExt;
use derive_more::IsVariant;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(IsVariant)]
pub enum ValueDifference<T> {
    LeftMissing(T),
    RightMissing(T),
    Different(T, T),
}

impl<'a, K, V> Diff<'a> for HashMap<K, V>
where
    K: Hash + Eq + 'a,
    V: Eq + 'a,
{
    type Diff = HashMap<&'a K, ValueDifference<&'a V>>;

    fn diff(left: &'a Self, right: &'a Self) -> Self::Diff {
        let mut result = HashMap::new();

        let mut keys = left.keys().collect_set();
        keys.extend(right.keys());

        for key in keys {
            match (left.get(key), right.get(key)) {
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
