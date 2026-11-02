pub trait Diff<'a> {
    type Diff;

    fn diff(left: &'a Self, right: &'a Self) -> Self::Diff;
}
