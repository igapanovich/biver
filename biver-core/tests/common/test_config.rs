use biver_core::Configuration;

pub fn create() -> Configuration {
    fn string_vec(collection: &[&str]) -> Vec<String> {
        collection.into_iter().map(|s| s.to_string()).collect()
    }

    Configuration {
        create_patch_command: string_vec(&[
            "xdelta3", "-D", "-e", "-s", "{old}", "{new}", "{patch}",
        ]),
        apply_patch_command: string_vec(&[
            "xdelta3", "-D", "-d", "-s", "{old}", "{patch}", "{new}",
        ]),
        file_type_rules: vec![],
    }
}
