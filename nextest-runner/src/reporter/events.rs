// Copyright (c) The nextest Contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::TestOutputDisplay;
use crate::{
    config::ScriptId,
    list::{TestInstance, TestInstanceId, TestList},
    runner::{
        ExecuteStatus, ExecutionResult, ExecutionStatuses, RetryData, SetupScriptExecuteStatus,
    },
    test_output::ChildExecutionOutput,
};
use chrono::{DateTime, FixedOffset};
use nextest_metadata::MismatchReason;
use quick_junit::ReportUuid;
use std::{fmt, time::Duration};

/// A test event.
///
/// Events are produced by a [`TestRunner`](crate::runner::TestRunner) and consumed by a
/// [`TestReporter`](crate::reporter::TestReporter).
#[derive(Clone, Debug)]
pub struct TestEvent<'a> {
    /// The time at which the event was generated, including the offset from UTC.
    pub timestamp: DateTime<FixedOffset>,

    /// The amount of time elapsed since the start of the test run.
    pub elapsed: Duration,

    /// The kind of test event this is.
    pub kind: TestEventKind<'a>,
}

/// The kind of test event this is.
///
/// Forms part of [`TestEvent`].
#[derive(Clone, Debug)]
pub enum TestEventKind<'a> {
    /// The test run started.
    RunStarted {
        /// The list of tests that will be run.
        ///
        /// The methods on the test list indicate the number of tests that will be run.
        test_list: &'a TestList<'a>,

        /// The UUID for this run.
        run_id: ReportUuid,

        /// The nextest profile chosen for this run.
        profile_name: String,

        /// The command-line arguments for the process.
        cli_args: Vec<String>,
    },

    /// A setup script started.
    SetupScriptStarted {
        /// The setup script index.
        index: usize,

        /// The total number of setup scripts.
        total: usize,

        /// The script ID.
        script_id: ScriptId,

        /// The command to run.
        command: &'a str,

        /// The arguments to the command.
        args: &'a [String],

        /// True if some output from the setup script is being passed through.
        no_capture: bool,
    },

    /// A setup script was slow.
    SetupScriptSlow {
        /// The script ID.
        script_id: ScriptId,

        /// The command to run.
        command: &'a str,

        /// The arguments to the command.
        args: &'a [String],

        /// The amount of time elapsed since the start of execution.
        elapsed: Duration,

        /// True if the script has hit its timeout and is about to be terminated.
        will_terminate: bool,
    },

    /// A setup script completed execution.
    SetupScriptFinished {
        /// The setup script index.
        index: usize,

        /// The total number of setup scripts.
        total: usize,

        /// The script ID.
        script_id: ScriptId,

        /// The command to run.
        command: &'a str,

        /// The arguments to the command.
        args: &'a [String],

        /// True if some output from the setup script was passed through.
        no_capture: bool,

        /// The execution status of the setup script.
        run_status: SetupScriptExecuteStatus,
    },

    // TODO: add events for BinaryStarted and BinaryFinished? May want a slightly different way to
    // do things, maybe a couple of reporter traits (one for the run as a whole and one for each
    // binary).
    /// A test started running.
    TestStarted {
        /// The test instance that was started.
        test_instance: TestInstance<'a>,

        /// Current run statistics so far.
        current_stats: RunStats,

        /// The number of tests currently running, including this one.
        running: usize,

        /// The cancel status of the run. This is None if the run is still ongoing.
        cancel_state: Option<CancelReason>,
    },

    /// A test was slower than a configured soft timeout.
    TestSlow {
        /// The test instance that was slow.
        test_instance: TestInstance<'a>,

        /// Retry data.
        retry_data: RetryData,

        /// The amount of time that has elapsed since the beginning of the test.
        elapsed: Duration,

        /// True if the test has hit its timeout and is about to be terminated.
        will_terminate: bool,
    },

    /// A test attempt failed and will be retried in the future.
    ///
    /// This event does not occur on the final run of a failing test.
    TestAttemptFailedWillRetry {
        /// The test instance that is being retried.
        test_instance: TestInstance<'a>,

        /// The status of this attempt to run the test. Will never be success.
        run_status: ExecuteStatus,

        /// The delay before the next attempt to run the test.
        delay_before_next_attempt: Duration,

        /// Whether failure outputs are printed out.
        failure_output: TestOutputDisplay,
    },

    /// A retry has started.
    TestRetryStarted {
        /// The test instance that is being retried.
        test_instance: TestInstance<'a>,

        /// Data related to retries.
        retry_data: RetryData,
    },

    /// A test finished running.
    TestFinished {
        /// The test instance that finished running.
        test_instance: TestInstance<'a>,

        /// Test setting for success output.
        success_output: TestOutputDisplay,

        /// Test setting for failure output.
        failure_output: TestOutputDisplay,

        /// Whether the JUnit report should store success output for this test.
        junit_store_success_output: bool,

        /// Whether the JUnit report should store failure output for this test.
        junit_store_failure_output: bool,

        /// Information about all the runs for this test.
        run_statuses: ExecutionStatuses,

        /// Current statistics for number of tests so far.
        current_stats: RunStats,

        /// The number of tests that are currently running, excluding this one.
        running: usize,

        /// The cancel status of the run. This is None if the run is still ongoing.
        cancel_state: Option<CancelReason>,
    },

    /// A test was skipped.
    TestSkipped {
        /// The test instance that was skipped.
        test_instance: TestInstance<'a>,

        /// The reason this test was skipped.
        reason: MismatchReason,
    },

    /// An information request was received.
    InfoStarted {
        /// The number of tasks currently running. This is the same as the
        /// number of expected responses.
        total: usize,

        /// Statistics for the run.
        run_stats: RunStats,
    },

    /// Information about a script or test was received.
    InfoResponse {
        /// The index of the response, starting from 0.
        index: usize,

        /// The total number of responses expected.
        total: usize,

        /// The response itself.
        response: InfoResponse<'a>,
    },

    /// An information request was completed.
    InfoFinished {
        /// The number of responses that were not received. In most cases, this
        /// is 0.
        missing: usize,
    },

    /// A cancellation notice was received.
    RunBeginCancel {
        /// The number of setup scripts still running.
        setup_scripts_running: usize,

        /// The number of tests still running.
        running: usize,

        /// The reason this run was cancelled.
        reason: CancelReason,
    },

    /// A SIGTSTP event was received and the run was paused.
    RunPaused {
        /// The number of setup scripts running.
        setup_scripts_running: usize,

        /// The number of tests currently running.
        running: usize,
    },

    /// A SIGCONT event was received and the run is being continued.
    RunContinued {
        /// The number of setup scripts that will be started up again.
        setup_scripts_running: usize,

        /// The number of tests that will be started up again.
        running: usize,
    },

    /// The test run finished.
    RunFinished {
        /// The unique ID for this run.
        run_id: ReportUuid,

        /// The time at which the run was started.
        start_time: DateTime<FixedOffset>,

        /// The amount of time it took for the tests to run.
        elapsed: Duration,

        /// Statistics for the run.
        run_stats: RunStats,
    },
}

/// Statistics for a test run.
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct RunStats {
    /// The total number of tests that were expected to be run at the beginning.
    ///
    /// If the test run is cancelled, this will be more than `finished_count` at the end.
    pub initial_run_count: usize,

    /// The total number of tests that finished running.
    pub finished_count: usize,

    /// The total number of setup scripts that were expected to be run at the beginning.
    ///
    /// If the test run is cancelled, this will be more than `finished_count` at the end.
    pub setup_scripts_initial_count: usize,

    /// The total number of setup scripts that finished running.
    pub setup_scripts_finished_count: usize,

    /// The number of setup scripts that passed.
    pub setup_scripts_passed: usize,

    /// The number of setup scripts that failed.
    pub setup_scripts_failed: usize,

    /// The number of setup scripts that encountered an execution failure.
    pub setup_scripts_exec_failed: usize,

    /// The number of setup scripts that timed out.
    pub setup_scripts_timed_out: usize,

    /// The number of tests that passed. Includes `passed_slow`, `flaky` and `leaky`.
    pub passed: usize,

    /// The number of slow tests that passed.
    pub passed_slow: usize,

    /// The number of tests that passed on retry.
    pub flaky: usize,

    /// The number of tests that failed.
    pub failed: usize,

    /// The number of failed tests that were slow.
    pub failed_slow: usize,

    /// The number of tests that timed out.
    pub timed_out: usize,

    /// The number of tests that passed but leaked handles.
    pub leaky: usize,

    /// The number of tests that encountered an execution failure.
    pub exec_failed: usize,

    /// The number of tests that were skipped.
    pub skipped: usize,
}

impl RunStats {
    /// Returns true if there are any failures recorded in the stats.
    pub fn has_failures(&self) -> bool {
        self.failed_setup_script_count() > 0 || self.failed_count() > 0
    }

    /// Returns count of setup scripts that did not pass.
    pub fn failed_setup_script_count(&self) -> usize {
        self.setup_scripts_failed + self.setup_scripts_exec_failed + self.setup_scripts_timed_out
    }

    /// Returns count of tests that did not pass.
    pub fn failed_count(&self) -> usize {
        self.failed + self.exec_failed + self.timed_out
    }

    /// Summarizes the stats as an enum at the end of a test run.
    pub fn summarize_final(&self) -> FinalRunStats {
        // Check for failures first. The order of setup scripts vs tests should not be important,
        // though we don't assert that here.
        if self.failed_setup_script_count() > 0 {
            FinalRunStats::Failed(RunStatsFailureKind::SetupScript)
        } else if self.setup_scripts_initial_count > self.setup_scripts_finished_count {
            FinalRunStats::Cancelled(RunStatsFailureKind::SetupScript)
        } else if self.failed_count() > 0 {
            FinalRunStats::Failed(RunStatsFailureKind::Test {
                initial_run_count: self.initial_run_count,
                not_run: self.initial_run_count.saturating_sub(self.finished_count),
            })
        } else if self.initial_run_count > self.finished_count {
            FinalRunStats::Cancelled(RunStatsFailureKind::Test {
                initial_run_count: self.initial_run_count,
                not_run: self.initial_run_count.saturating_sub(self.finished_count),
            })
        } else if self.finished_count == 0 {
            FinalRunStats::NoTestsRun
        } else {
            FinalRunStats::Success
        }
    }

    pub(crate) fn on_setup_script_finished(&mut self, status: &SetupScriptExecuteStatus) {
        self.setup_scripts_finished_count += 1;

        match status.result {
            ExecutionResult::Pass | ExecutionResult::Leak => {
                self.setup_scripts_passed += 1;
            }
            ExecutionResult::Fail { .. } => {
                self.setup_scripts_failed += 1;
            }
            ExecutionResult::ExecFail => {
                self.setup_scripts_exec_failed += 1;
            }
            ExecutionResult::Timeout => {
                self.setup_scripts_timed_out += 1;
            }
        }
    }

    pub(crate) fn on_test_finished(&mut self, run_statuses: &ExecutionStatuses) {
        self.finished_count += 1;
        // run_statuses is guaranteed to have at least one element.
        // * If the last element is success, treat it as success (and possibly flaky).
        // * If the last element is a failure, use it to determine fail/exec fail.
        // Note that this is different from what Maven Surefire does (use the first failure):
        // https://maven.apache.org/surefire/maven-surefire-plugin/examples/rerun-failing-tests.html
        //
        // This is not likely to matter much in practice since failures are likely to be of the
        // same type.
        let last_status = run_statuses.last_status();
        match last_status.result {
            ExecutionResult::Pass => {
                self.passed += 1;
                if last_status.is_slow {
                    self.passed_slow += 1;
                }
                if run_statuses.len() > 1 {
                    self.flaky += 1;
                }
            }
            ExecutionResult::Leak => {
                self.passed += 1;
                self.leaky += 1;
                if last_status.is_slow {
                    self.passed_slow += 1;
                }
                if run_statuses.len() > 1 {
                    self.flaky += 1;
                }
            }
            ExecutionResult::Fail { .. } => {
                self.failed += 1;
                if last_status.is_slow {
                    self.failed_slow += 1;
                }
            }
            ExecutionResult::Timeout => self.timed_out += 1,
            ExecutionResult::ExecFail => self.exec_failed += 1,
        }
    }
}

/// A type summarizing the possible outcomes of a test run.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FinalRunStats {
    /// The test run was successful, or is successful so far.
    Success,

    /// The test run was successful, or is successful so far, but no tests were selected to run.
    NoTestsRun,

    /// The test run was cancelled.
    Cancelled(RunStatsFailureKind),

    /// At least one test failed.
    Failed(RunStatsFailureKind),
}

/// A type summarizing the step at which a test run failed.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RunStatsFailureKind {
    /// The run was interrupted during setup script execution.
    SetupScript,

    /// The run was interrupted during test execution.
    Test {
        /// The total number of tests scheduled.
        initial_run_count: usize,

        /// The number of tests not run, or for a currently-executing test the number queued up to
        /// run.
        not_run: usize,
    },
}

// Note: the order here matters -- it indicates severity of cancellation
/// The reason why a test run is being cancelled.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub enum CancelReason {
    /// A setup script failed.
    SetupScriptFailure,

    /// A test failed and --no-fail-fast wasn't specified.
    TestFailure,

    /// An error occurred while reporting results.
    ReportError,

    /// A termination signal (on Unix, SIGTERM or SIGHUP) was received.
    Signal,

    /// An interrupt (on Unix, Ctrl-C) was received.
    Interrupt,
}

impl CancelReason {
    pub(crate) fn to_static_str(self) -> &'static str {
        match self {
            CancelReason::SetupScriptFailure => "setup script failure",
            CancelReason::TestFailure => "test failure",
            CancelReason::ReportError => "reporting error",
            CancelReason::Signal => "signal",
            CancelReason::Interrupt => "interrupt",
        }
    }
}
/// The kind of unit of work that nextest is executing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    /// A test.
    Test,

    /// A script (e.g. a setup script).
    Script,
}

impl UnitKind {
    pub(crate) const WAITING_ON_TEST_MESSAGE: &str = "waiting on test process";
    pub(crate) const WAITING_ON_SCRIPT_MESSAGE: &str = "waiting on script process";

    pub(crate) const EXECUTING_TEST_MESSAGE: &str = "executing test";
    pub(crate) const EXECUTING_SCRIPT_MESSAGE: &str = "executing script";

    pub(crate) fn waiting_on_message(&self) -> &'static str {
        match self {
            UnitKind::Test => Self::WAITING_ON_TEST_MESSAGE,
            UnitKind::Script => Self::WAITING_ON_SCRIPT_MESSAGE,
        }
    }

    pub(crate) fn executing_message(&self) -> &'static str {
        match self {
            UnitKind::Test => Self::EXECUTING_TEST_MESSAGE,
            UnitKind::Script => Self::EXECUTING_SCRIPT_MESSAGE,
        }
    }
}

impl fmt::Display for UnitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnitKind::Script => write!(f, "script"),
            UnitKind::Test => write!(f, "test"),
        }
    }
}

/// A response to an information request.
#[derive(Clone, Debug)]
pub enum InfoResponse<'a> {
    /// A setup script's response.
    SetupScript(SetupScriptInfoResponse<'a>),

    /// A test's response.
    Test(TestInfoResponse<'a>),
}

/// A setup script's response to an information request.
#[derive(Clone, Debug)]
pub struct SetupScriptInfoResponse<'a> {
    /// The identifier of the setup script instance.
    pub script_id: ScriptId,

    /// The command to run.
    pub command: &'a str,

    /// The list of arguments to the command.
    pub args: &'a [String],

    /// The state of the setup script.
    pub state: UnitState,

    /// Output obtained from the setup script.
    pub output: ChildExecutionOutput,
}

/// A test's response to an information request.
#[derive(Clone, Debug)]
pub struct TestInfoResponse<'a> {
    /// The test instance that the information is about.
    pub test_instance: TestInstanceId<'a>,

    /// Information about retries.
    pub retry_data: RetryData,

    /// The state of the test.
    pub state: UnitState,

    /// Output obtained from the test.
    pub output: ChildExecutionOutput,
}

/// The current state of a test or script process: running, exiting, or
/// terminating.
///
/// Part of information response requests.
#[derive(Clone, Debug)]
pub enum UnitState {
    /// The unit is currently running.
    Running {
        /// The process ID.
        pid: u32,

        /// The amount of time the unit has been running.
        time_taken: Duration,

        /// `Some` if the test is marked as slow, along with the duration after
        /// which it was marked as slow.
        slow_after: Option<Duration>,
    },

    /// The test has finished running, and is currently in the process of
    /// exiting.
    Exiting {
        /// The process ID.
        pid: u32,

        /// The amount of time the unit ran for.
        time_taken: Duration,

        /// `Some` if the unit is marked as slow, along with the duration after
        /// which it was marked as slow.
        slow_after: Option<Duration>,

        /// The tentative execution result before leaked status is determined.
        ///
        /// None means that the exit status could not be read, and should be
        /// treated as a failure.
        tentative_result: Option<ExecutionResult>,

        /// How long has been spent waiting for the process to exit.
        waiting_duration: Duration,

        /// How much longer nextest will wait until the test is marked leaky.
        remaining: Duration,
    },

    /// The child process is being terminated by nextest.
    Terminating(UnitTerminatingState),

    /// The unit has finished running and the process has exited.
    Exited {
        /// The result of executing the unit.
        result: ExecutionResult,

        /// The amount of time the unit ran for.
        time_taken: Duration,

        /// `Some` if the unit is marked as slow, along with the duration after
        /// which it was marked as slow.
        slow_after: Option<Duration>,
    },

    /// A delay is being waited out before the next attempt of the test is
    /// started. (Only relevant for tests.)
    DelayBeforeNextAttempt {
        /// The previous execution result.
        previous_result: ExecutionResult,

        /// Whether the previous attempt was marked as slow.
        previous_slow: bool,

        /// How long has been spent waiting so far.
        waiting_duration: Duration,

        /// How much longer nextest will wait until retrying the test.
        remaining: Duration,
    },
}

impl UnitState {
    /// Returns true if the state has a valid output attached to it.
    pub fn has_valid_output(&self) -> bool {
        match self {
            UnitState::Running { .. }
            | UnitState::Exiting { .. }
            | UnitState::Terminating(_)
            | UnitState::Exited { .. } => true,
            UnitState::DelayBeforeNextAttempt { .. } => false,
        }
    }
}

/// The current terminating state of a test or script process.
///
/// Part of [`UnitState::Terminating`].
#[derive(Clone, Debug)]
pub struct UnitTerminatingState {
    /// The process ID.
    pub pid: u32,

    /// The amount of time the unit ran for.
    pub time_taken: Duration,

    /// The reason for the termination.
    pub reason: UnitTerminateReason,

    /// The method by which the process is being terminated.
    pub method: UnitTerminateMethod,

    /// How long has been spent waiting for the process to exit.
    pub waiting_duration: Duration,

    /// How much longer nextest will wait until a kill command is sent to the process.
    pub remaining: Duration,
}

/// The reason for a script or test being forcibly terminated by nextest.
///
/// Part of information response requests.
#[derive(Clone, Copy, Debug)]
pub enum UnitTerminateReason {
    /// The unit is being terminated due to a test timeout being hit.
    Timeout,

    /// The unit is being terminated due to nextest receiving a signal.
    Signal,

    /// The unit is being terminated due to an interrupt (i.e. Ctrl-C).
    Interrupt,
}

impl fmt::Display for UnitTerminateReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnitTerminateReason::Timeout => write!(f, "timeout"),
            UnitTerminateReason::Signal => write!(f, "signal"),
            UnitTerminateReason::Interrupt => write!(f, "interrupt"),
        }
    }
}

/// The way in which a script or test is being forcibly terminated by nextest.
#[derive(Clone, Copy, Debug)]
pub enum UnitTerminateMethod {
    /// The unit is being terminated by sending a signal.
    #[cfg(unix)]
    Signal(UnitTerminateSignal),

    /// The unit is being terminated by terminating the Windows job object.
    #[cfg(windows)]
    JobObject,

    /// A fake method used for testing.
    #[cfg(test)]
    Fake,
}

#[cfg(unix)]
/// The signal that is or was sent to terminate a script or test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitTerminateSignal {
    /// The unit is being terminated by sending a SIGINT.
    Interrupt,

    /// The unit is being terminated by sending a SIGTERM signal.
    Term,

    /// The unit is being terminated by sending a SIGHUP signal.
    Hangup,

    /// The unit is being terminated by sending a SIGQUIT signal.
    Quit,

    /// The unit is being terminated by sending a SIGKILL signal.
    Kill,
}

#[cfg(unix)]
impl fmt::Display for UnitTerminateSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnitTerminateSignal::Interrupt => write!(f, "SIGINT"),
            UnitTerminateSignal::Term => write!(f, "SIGTERM"),
            UnitTerminateSignal::Hangup => write!(f, "SIGHUP"),
            UnitTerminateSignal::Quit => write!(f, "SIGQUIT"),
            UnitTerminateSignal::Kill => write!(f, "SIGKILL"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_success() {
        assert_eq!(
            RunStats::default().summarize_final(),
            FinalRunStats::NoTestsRun,
            "empty run => no tests run"
        );
        assert_eq!(
            RunStats {
                initial_run_count: 42,
                finished_count: 42,
                ..RunStats::default()
            }
            .summarize_final(),
            FinalRunStats::Success,
            "initial run count = final run count => success"
        );
        assert_eq!(
            RunStats {
                initial_run_count: 42,
                finished_count: 41,
                ..RunStats::default()
            }
            .summarize_final(),
            FinalRunStats::Cancelled(RunStatsFailureKind::Test {
                initial_run_count: 42,
                not_run: 1
            }),
            "initial run count > final run count => cancelled"
        );
        assert_eq!(
            RunStats {
                initial_run_count: 42,
                finished_count: 42,
                failed: 1,
                ..RunStats::default()
            }
            .summarize_final(),
            FinalRunStats::Failed(RunStatsFailureKind::Test {
                initial_run_count: 42,
                not_run: 0
            }),
            "failed => failure"
        );
        assert_eq!(
            RunStats {
                initial_run_count: 42,
                finished_count: 42,
                exec_failed: 1,
                ..RunStats::default()
            }
            .summarize_final(),
            FinalRunStats::Failed(RunStatsFailureKind::Test {
                initial_run_count: 42,
                not_run: 0
            }),
            "exec failed => failure"
        );
        assert_eq!(
            RunStats {
                initial_run_count: 42,
                finished_count: 42,
                timed_out: 1,
                ..RunStats::default()
            }
            .summarize_final(),
            FinalRunStats::Failed(RunStatsFailureKind::Test {
                initial_run_count: 42,
                not_run: 0
            }),
            "timed out => failure"
        );
        assert_eq!(
            RunStats {
                initial_run_count: 42,
                finished_count: 42,
                skipped: 1,
                ..RunStats::default()
            }
            .summarize_final(),
            FinalRunStats::Success,
            "skipped => not considered a failure"
        );

        assert_eq!(
            RunStats {
                setup_scripts_initial_count: 2,
                setup_scripts_finished_count: 1,
                ..RunStats::default()
            }
            .summarize_final(),
            FinalRunStats::Cancelled(RunStatsFailureKind::SetupScript),
            "setup script failed => failure"
        );

        assert_eq!(
            RunStats {
                setup_scripts_initial_count: 2,
                setup_scripts_finished_count: 2,
                setup_scripts_failed: 1,
                ..RunStats::default()
            }
            .summarize_final(),
            FinalRunStats::Failed(RunStatsFailureKind::SetupScript),
            "setup script failed => failure"
        );
        assert_eq!(
            RunStats {
                setup_scripts_initial_count: 2,
                setup_scripts_finished_count: 2,
                setup_scripts_exec_failed: 1,
                ..RunStats::default()
            }
            .summarize_final(),
            FinalRunStats::Failed(RunStatsFailureKind::SetupScript),
            "setup script exec failed => failure"
        );
        assert_eq!(
            RunStats {
                setup_scripts_initial_count: 2,
                setup_scripts_finished_count: 2,
                setup_scripts_timed_out: 1,
                ..RunStats::default()
            }
            .summarize_final(),
            FinalRunStats::Failed(RunStatsFailureKind::SetupScript),
            "setup script timed out => failure"
        );
        assert_eq!(
            RunStats {
                setup_scripts_initial_count: 2,
                setup_scripts_finished_count: 2,
                setup_scripts_passed: 2,
                ..RunStats::default()
            }
            .summarize_final(),
            FinalRunStats::NoTestsRun,
            "setup scripts passed => success, but no tests run"
        );
    }
}
