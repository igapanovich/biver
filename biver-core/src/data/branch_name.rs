use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

impl BranchName {
    pub fn new(branch_name: String) -> Result<Self, (String, ValidationError)> {
        match Self::validate(&branch_name) {
            Ok(()) => Ok(BranchName(branch_name)),
            Err(e) => Err((branch_name, e)),
        }
    }

    pub fn new_cloned(branch_name: &str) -> Result<Self, ValidationError> {
        match Self::validate(&branch_name) {
            Ok(()) => Ok(BranchName(branch_name.to_string())),
            Err(e) => Err(e),
        }
    }

    pub fn validate(branch_name: &str) -> Result<(), ValidationError> {
        if branch_name.is_empty() {
            return Err(ValidationError::MustNotBeEmpty);
        }

        let all_chars_valid = branch_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

        if !all_chars_valid {
            return Err(ValidationError::InvalidCharacters);
        }

        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for BranchName {
    fn default() -> BranchName {
        BranchName::new("main".to_string()).unwrap()
    }
}

impl Debug for BranchName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for BranchName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.clone())
    }
}

#[derive(Debug)]
pub enum ValidationError {
    MustNotBeEmpty,
    InvalidCharacters,
}
