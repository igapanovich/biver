use proptest::sample::Index;

pub trait IndexExt {
    fn debug_fraction(&self) -> String;

    fn get_copied<T: Copy>(&self, slice: &[T]) -> T;
}

impl IndexExt for Index {
    fn debug_fraction(&self) -> String {
        let value = self.index(101);

        if value == 100 {
            return "1.00".to_string();
        }

        let mut result = String::with_capacity(4);
        result.push_str("0.");
        result.push_str(&format!("{:02}", value));
        result
    }

    fn get_copied<T: Copy>(&self, slice: &[T]) -> T {
        *self.get(slice)
    }
}
