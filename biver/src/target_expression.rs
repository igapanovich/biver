use biver_core::data::{BranchName, VersionId};
use biver_core::repository::Session;
use itertools::Itertools;
use std::str::FromStr;

pub const UNCOMMITTED_CHANGES_EXPRESSION: &str = "*";

pub enum VersionOrUncommitted {
    Version(VersionId),
    Uncommitted,
}

pub enum BranchOrVersion {
    Branch(BranchName),
    Version(VersionId),
}

pub fn resolve_version_or_uncommitted(
    session: &Session,
    target_expr: &str,
) -> Option<VersionOrUncommitted> {
    if target_expr == UNCOMMITTED_CHANGES_EXPRESSION {
        Some(VersionOrUncommitted::Uncommitted)
    } else {
        resolve_version(session, target_expr).map(VersionOrUncommitted::Version)
    }
}

pub fn resolve_branch_or_version(session: &Session, target_expr: &str) -> Option<BranchOrVersion> {
    if let Some(branch_name) = resolve_as_branch_name(session, target_expr) {
        return Some(BranchOrVersion::Branch(branch_name));
    }

    if let Some(id) = resolve_as_bs58(target_expr) {
        return Some(BranchOrVersion::Version(id));
    }

    if let Some(id) = resolve_as_relative_to_head_expr(session, target_expr) {
        return Some(BranchOrVersion::Version(id));
    }

    if let Some(id) = resolve_as_nickname(session, target_expr) {
        return Some(BranchOrVersion::Version(id));
    }

    None
}

pub fn resolve_version(session: &Session, target_expr: &str) -> Option<VersionId> {
    match resolve_branch_or_version(session, target_expr) {
        Some(BranchOrVersion::Branch(branch_name)) => {
            Some(session.tree().branch_tip_ids()[&branch_name])
        }
        Some(BranchOrVersion::Version(version_id)) => Some(version_id),
        None => None,
    }
}

pub fn resolve_as_bs58(bs58: &str) -> Option<VersionId> {
    VersionId::from_bs58(bs58)
}

fn resolve_as_relative_to_head_expr(session: &Session, expr: &str) -> Option<VersionId> {
    if !expr.starts_with('@') {
        return None;
    }

    if expr.len() == 1 {
        return Some(session.tree().head_version().id);
    }

    if let Ok(offset) = usize::from_str(&expr[1..]) {
        if let Some(version) = session.tree().head_and_ancestors().skip(offset).next() {
            return Some(version.id);
        }
    }

    None
}

fn resolve_as_branch_name(session: &Session, branch_name: &str) -> Option<BranchName> {
    let Ok(branch_name) = BranchName::new_cloned(branch_name) else {
        return None;
    };

    if session.tree().branch_names().contains(&branch_name) {
        return Some(branch_name);
    }

    None
}

fn resolve_as_nickname(session: &Session, nickname: &str) -> Option<VersionId> {
    let mut versions = session.tree().versions().collect_vec();
    versions.sort_by(|a, b| b.creation_time.cmp(&a.creation_time));

    if let Some(version) = versions
        .iter()
        .filter(|v| nickname_matches(&v.nickname(), nickname))
        .next()
    {
        return Some(version.id);
    }

    None
}

fn nickname_matches(nickname: &str, tested_str: &str) -> bool {
    if nickname.eq_ignore_ascii_case(tested_str) {
        return true;
    }

    fn nickname_without_dash_matches(nickname: &str, tested_str: &str) -> bool {
        let pairs = nickname
            .chars()
            .filter(|c| c != &'-')
            .zip(tested_str.chars());

        let mut zip_length = 0;

        for (nickname_char, input_char) in pairs {
            zip_length += 1;

            if !nickname_char.eq_ignore_ascii_case(&input_char) {
                return false;
            }
        }

        zip_length == tested_str.len()
    }

    if nickname_without_dash_matches(nickname, tested_str) {
        return true;
    }

    fn nickname_initials_match(nickname: &str, tested_str: &str) -> bool {
        if tested_str.len() != 2 {
            return false;
        }

        let input_initials_first = tested_str.chars().nth(0).unwrap();
        let input_initials_second = tested_str.chars().nth(1).unwrap();

        let index_of_dash = nickname.find('-').unwrap();
        let nickname_initials_first = nickname.chars().nth(0).unwrap();
        let nickname_initials_second = nickname.chars().nth(index_of_dash + 1).unwrap();

        input_initials_first.eq_ignore_ascii_case(&nickname_initials_first)
            && input_initials_second.eq_ignore_ascii_case(&nickname_initials_second)
    }

    nickname_initials_match(nickname, tested_str)
}
