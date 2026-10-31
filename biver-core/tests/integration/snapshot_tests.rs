use crate::helpers::test_env::TestEnv;
use crate::time_control_environment::TimeControlEnvironment;
use biver_core::data::{Snapshot, Version};
use biver_std_environment::StdEnvironment;
use rstest::rstest;
use std::error::Error;
use std::time::Duration;
use std::{assert_matches, fs};

#[rstest]
fn cannot_start_session_with_unavailable_snapshot() -> Result<(), Box<dyn Error>> {
    let test_env = TestEnv::new(StdEnvironment);

    let session_errors = vec![
        test_env.start_session_with(Snapshot::Snapshot2).err(),
        test_env.start_session_with(Snapshot::Snapshot3).err(),
        test_env.start_session_with(Snapshot::Snapshot4).err(),
        test_env.start_session_with(Snapshot::Snapshot5).err(),
        test_env.start_session_with(Snapshot::Snapshot6).err(),
        test_env.start_session_with(Snapshot::Snapshot7).err(),
        test_env.start_session_with(Snapshot::Snapshot8).err(),
    ];

    for e in session_errors {
        assert_matches!(e, Some(biver_core::Error::SnapshotUnavailable));
    }

    Ok(())
}

#[rstest]
#[case(0, Snapshot::Snapshot1)]
#[case(1, Snapshot::Snapshot2)]
#[case(2, Snapshot::Snapshot3)]
#[case(3, Snapshot::Snapshot4)]
#[case(4, Snapshot::Snapshot5)]
#[case(5, Snapshot::Snapshot6)]
#[case(6, Snapshot::Snapshot7)]
#[case(7, Snapshot::Snapshot8)]
fn snapshot_is_available_after_n_changes(
    #[case] change_count: u32,
    #[case] snapshot: Snapshot,
) -> Result<(), Box<dyn Error>> {
    let mut test_env = TestEnv::new(StdEnvironment);

    make_commits(change_count, &mut test_env);

    assert!(test_env.start_session_with(snapshot).is_ok());

    Ok(())
}

#[rstest]
#[case(Duration::from_secs(10), Snapshot::Snapshot1)]
#[case(Duration::from_mins(5), Snapshot::Snapshot2)]
#[case(Duration::from_mins(30), Snapshot::Snapshot3)]
#[case(Duration::from_hours(1), Snapshot::Snapshot4)]
#[case(Duration::from_hours(5), Snapshot::Snapshot5)]
#[case(Duration::from_hours(24), Snapshot::Snapshot6)]
#[case(Duration::from_hours(24 * 3), Snapshot::Snapshot7)]
#[case(Duration::from_hours(24 * 7), Snapshot::Snapshot8)]
fn snapshot_is_updated_after_its_retention_period(
    #[case] retention_period: Duration,
    #[case] snapshot: Snapshot,
) -> Result<(), Box<dyn Error>> {
    let mut env = TimeControlEnvironment::new();
    let mut test_env = TestEnv::new(env.clone());

    make_warmup_commits(&mut test_env, &mut env);

    let state1 = get_snapshot_state(&test_env, snapshot);

    env.advance_time(retention_period - Duration::from_millis(1));
    make_commits(1, &mut test_env);
    let state2 = get_snapshot_state(&test_env, snapshot);

    assert_eq!(state1, state2);

    env.advance_time(Duration::from_millis(1));
    make_commits(1, &mut test_env);
    let state3 = get_snapshot_state(&test_env, snapshot);

    assert_ne!(state2, state3);

    Ok(())
}

fn get_snapshot_state(test_env: &TestEnv, snapshot: Snapshot) -> Vec<Version> {
    let session = test_env.start_session_with(snapshot).unwrap();
    session.tree().versions().cloned().collect()
}

fn make_warmup_commits(test_env: &mut TestEnv, env: &mut TimeControlEnvironment) {
    make_commits_base(8, test_env, &mut TimeControl::AdvanceTime(env));
    env.advance_time(Duration::from_hours(24 * 7));
    make_commits_base(8, test_env, &mut TimeControl::NoControl);
}

fn make_commits(n: u32, test_env: &mut TestEnv) {
    make_commits_base(n, test_env, &mut TimeControl::NoControl);
}

fn make_commits_base(n: u32, test_env: &mut TestEnv, time_control: &mut TimeControl) {
    let mut session = test_env.start_session().unwrap();

    for _ in 0..n {
        if let TimeControl::AdvanceTime(env) = time_control {
            env.advance_time(Duration::from_hours(24 * 5));
        }

        let sample = test_env.next_sample();
        fs::write(test_env.versioned_file_path(), sample).unwrap();

        session.commit(String::new()).unwrap().unwrap_ok();
    }
}

enum TimeControl<'a> {
    AdvanceTime(&'a mut TimeControlEnvironment),
    NoControl,
}
