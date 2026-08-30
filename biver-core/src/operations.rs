use crate::data::{Repository, Version, VersionId};
use crate::error::Error;
use crate::repository_paths::RepositoryPaths;
use std::fs;
use std::str::FromStr;

pub mod branch;

pub mod amend;
pub mod check_out;
pub mod commit;
pub mod discard;
pub mod has_uncommitted_changes;
pub mod init;
pub mod read_repository;
pub mod reset;
pub mod resolve_version;
pub mod restore;
pub mod reword;

pub use amend::amend;
pub use check_out::check_out;
pub use commit::commit;
pub use discard::discard;
pub use has_uncommitted_changes::has_uncommitted_changes;
pub use init::init;
pub use read_repository::read_repository;
pub use reset::reset;
pub use resolve_version::resolve_version;
pub use restore::restore;
pub use reword::reword;

const DEFAULT_BRANCH: &str = "main";

enum TargetResult<'b, 'v> {
    Branch(&'b str),
    Version(&'v Version),
    Invalid,
}

fn resolve_target<'b, 'v>(repo: &'v Repository, target: &'b str) -> TargetResult<'b, 'v> {
    if target.is_empty() {
        return TargetResult::Invalid;
    }

    // As branch name
    if repo.branches.contains_key(target) {
        return TargetResult::Branch(target);
    }

    // As version ID
    let target_as_version_id = VersionId::from_bs58(target);

    if let Some(target_as_version_id) = target_as_version_id {
        let version = repo.versions.iter().find(|v| v.id == target_as_version_id);
        if let Some(version) = version {
            return TargetResult::Version(version);
        }
    }

    // As offset
    if target == "~" {
        return TargetResult::Version(repo.head_version());
    }

    if target.chars().nth(0) == Some('~')
        && let Ok(offset) = usize::from_str(&target[1..])
    {
        let target_version = repo.iter_head_and_ancestors().nth(offset);
        return match target_version {
            None => TargetResult::Invalid,
            Some(target_version) => TargetResult::Version(target_version),
        };
    }

    // As version nickname
    let mut versions: Vec<_> = repo.versions.iter().collect();
    versions.sort_by(|a, b| b.creation_time.cmp(&a.creation_time));

    let version = versions
        .iter()
        .find(|v| nickname_matches(&v.nickname, target));

    if let Some(version) = version {
        return TargetResult::Version(version);
    }

    TargetResult::Invalid
}

fn resolve_target_strict_mut<'v>(
    repo: &'v mut Repository,
    target: &str,
) -> Option<&'v mut Version> {
    if target.is_empty() {
        return None;
    }

    let target_as_version_id = VersionId::from_bs58(target);

    if let Some(target_as_version_id) = target_as_version_id {
        let version = repo
            .versions
            .iter_mut()
            .find(|v| v.id == target_as_version_id);
        if let Some(version) = version {
            return Some(version);
        }
    }

    None
}

fn resolve_version_id_target<'v>(repo: &'v Repository, target: &str) -> Option<&'v Version> {
    if target.is_empty() {
        return None;
    }

    let target_as_version_id = VersionId::from_bs58(target);

    if let Some(target_as_version_id) = target_as_version_id {
        let version = repo.versions.iter().find(|v| v.id == target_as_version_id);
        if let Some(version) = version {
            return Some(version);
        }
    }

    None
}

fn nickname_matches(nickname: &str, input: &str) -> bool {
    if nickname.eq_ignore_ascii_case(input) {
        return true;
    }

    fn nickname_without_dash_matches(nickname: &str, input: &str) -> bool {
        let pairs = nickname.chars().filter(|c| c != &'-').zip(input.chars());

        let mut zip_length = 0;

        for (nickname_char, input_char) in pairs {
            zip_length += 1;

            if !nickname_char.eq_ignore_ascii_case(&input_char) {
                return false;
            }
        }

        zip_length == input.len()
    }

    if nickname_without_dash_matches(nickname, input) {
        return true;
    }

    fn nickname_initials_match(nickname: &str, input: &str) -> bool {
        if input.len() != 2 {
            return false;
        }

        let input_initials_first = input.chars().nth(0).unwrap();
        let input_initials_second = input.chars().nth(1).unwrap();

        let index_of_dash = nickname.find('-').unwrap();
        let nickname_initials_first = nickname.chars().nth(0).unwrap();
        let nickname_initials_second = nickname.chars().nth(index_of_dash + 1).unwrap();

        input_initials_first.eq_ignore_ascii_case(&nickname_initials_first)
            && input_initials_second.eq_ignore_ascii_case(&nickname_initials_second)
    }

    nickname_initials_match(nickname, input)
}

fn content_blob_file_name(version_id: VersionId) -> String {
    version_id.to_file_name() + "_content"
}

fn preview_blob_file_name(version_id: VersionId) -> String {
    version_id.to_file_name() + "_preview"
}

fn valid_branch_name(branch_name: &str) -> bool {
    branch_name.len() > 0
        && branch_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn should_convert_patch_to_full(
    paths: &RepositoryPaths,
    repo: &Repository,
    parent_id: VersionId,
    new_patch_length: u64,
) -> Result<bool, Error> {
    let versioned_file_length = fs::metadata(&paths.versioned_file)?.len();

    let mut patch_chain_length = new_patch_length;

    let preceding_patch_chain = repo
        .iter_version_and_ancestors(parent_id)
        .take_while(|v| v.content_blob_kind.is_patch());

    for patch_version in preceding_patch_chain {
        patch_chain_length +=
            fs::metadata(paths.file_path(&patch_version.content_blob_file_name))?.len();
    }

    let threshold = versioned_file_length as f64 * 0.65;

    Ok(patch_chain_length > threshold as u64)
}
