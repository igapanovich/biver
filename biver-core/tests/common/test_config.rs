use biver_core::Config;

pub fn create() -> Config {
    fn string_vec(collection: &[&str]) -> Vec<String> {
        collection.into_iter().map(|s| s.to_string()).collect()
    }

    Config {
        create_patch_command: string_vec(&[
            "xdelta3", "-D", "-e", "-s", "{old}", "{new}", "{patch}",
        ]),
        apply_patch_command: string_vec(&[
            "xdelta3", "-D", "-d", "-s", "{old}", "{patch}", "{new}",
        ]),
        preview_command: None,
    }
}
