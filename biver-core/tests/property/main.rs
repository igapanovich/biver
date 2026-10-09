#[path = "../common/test_dir.rs"]
mod test_dir;

#[path = "../common/test_config.rs"]
mod test_config;

mod helpers;

mod check_out_tests;
mod commit_tests;
mod discard_tests;
mod reword_tests;
mod init_tests;

const DEFAULT_CASE_COUNT: u32 = 256;
const GROUP_CASE_MULTIPLIER: u32 = 2;

/*

"commit does not modify versioned file"
    + "successful commit does not leave uncommitted changes"
    + "successful check_out does not leave uncommitted changes"
=> "version captures the state of the versioned file at the time of commit"

*/
