use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

impl Default for BranchName {
    fn default() -> BranchName {
        BranchName::new("main".to_string()).unwrap()
    }
}

impl BranchName {
    pub fn new(branch_name: String) -> Result<Self, ValidationError> {
        if branch_name.is_empty() {
            return Err(ValidationError::MustNotBeEmpty);
        }

        let all_chars_valid = branch_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

        if !all_chars_valid {
            return Err(ValidationError::InvalidCharacters);
        }

        Ok(BranchName(branch_name))
    }
}

#[derive(Debug)]
pub enum ValidationError {
    MustNotBeEmpty,
    InvalidCharacters,
}
