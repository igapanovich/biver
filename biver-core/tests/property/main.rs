#[path = "../common/test_dir.rs"]
mod test_dir;

#[path = "../common/test_config.rs"]
mod test_config;

mod helpers;

mod amend_tests;
mod check_out_branch_tests;
mod check_out_version_tests;
mod commit_tests;
mod create_branch_tests;
mod discard_tests;
mod init_tests;
mod remove_branch_tests;
mod rename_branch_tests;
mod restore_version_tests;
mod soft_reset_tests;
mod update_description_tests;
