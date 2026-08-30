pub struct Configuration {
    pub create_patch_command: Vec<String>,
    pub apply_patch_command: Vec<String>,
    pub file_type_rules: Vec<FileTypeRule>,
}

pub struct FileTypeRule {
    pub extensions: Vec<String>,
    pub preview_command: Option<Vec<String>>,
}
