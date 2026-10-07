#[path = "../common/test_dir.rs"]
mod test_dir;

#[path = "../common/test_config.rs"]
mod test_config;

mod helpers;

mod check_out_tests;
mod commit_tests;

const DEFAULT_CASE_COUNT: u32 = 256;
const GROUP_CASE_MULTIPLIER: u32 = 2;
