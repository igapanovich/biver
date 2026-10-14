macro_rules! set {
    () => {
        ::std::collections::HashSet::new()
    };

    ($($x:expr),+ $(,)?) => {{
        let mut set = ::std::collections::HashSet::new();
        $(
            set.insert($x);
        )+
        set
    }};
}

pub(crate) use set;
