// Copyright (c) The nextest Contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::CommandError;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::PathBuf,
    process::Command,
};

/// Command builder for `cargo nextest list`.
#[derive(Clone, Debug, Default)]
pub struct ListCommand {
    cargo_path: Option<Box<Utf8Path>>,
    manifest_path: Option<Box<Utf8Path>>,
    current_dir: Option<Box<Utf8Path>>,
    args: Vec<Box<str>>,
}

impl ListCommand {
    /// Creates a new `ListCommand`.
    ///
    /// This command runs `cargo nextest list`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Path to `cargo` executable. If not set, this will use the the `$CARGO` environment variable, and
    /// if that is not set, will simply be `cargo`.
    pub fn cargo_path(&mut self, path: impl Into<Utf8PathBuf>) -> &mut Self {
        self.cargo_path = Some(path.into().into());
        self
    }

    /// Path to `Cargo.toml`.
    pub fn manifest_path(&mut self, path: impl Into<Utf8PathBuf>) -> &mut Self {
        self.manifest_path = Some(path.into().into());
        self
    }

    /// Current directory of the `cargo nextest list` process.
    pub fn current_dir(&mut self, path: impl Into<Utf8PathBuf>) -> &mut Self {
        self.current_dir = Some(path.into().into());
        self
    }

    /// Adds an argument to the end of `cargo nextest list`.
    pub fn add_arg(&mut self, arg: impl Into<String>) -> &mut Self {
        self.args.push(arg.into().into());
        self
    }

    /// Adds several arguments to the end of `cargo nextest list`.
    pub fn add_args(&mut self, args: impl IntoIterator<Item = impl Into<String>>) -> &mut Self {
        for arg in args {
            self.add_arg(arg.into());
        }
        self
    }

    /// Builds a command for `cargo nextest list`. This is the first part of the work of [`self.exec`].
    pub fn cargo_command(&self) -> Command {
        let cargo_path: PathBuf = self.cargo_path.as_ref().map_or_else(
            || std::env::var_os("CARGO").map_or("cargo".into(), PathBuf::from),
            |path| PathBuf::from(path.as_std_path()),
        );

        let mut command = Command::new(&cargo_path);
        if let Some(path) = &self.manifest_path.as_deref() {
            command.args(["--manifest-path", path.as_str()]);
        }
        if let Some(current_dir) = &self.current_dir.as_deref() {
            command.current_dir(current_dir);
        }

        command.args(["nextest", "list", "--format=json"]);

        command.args(self.args.iter().map(|s| s.as_ref()));
        command
    }

    /// Executes `cargo nextest list` and parses the output into a [`TestListSummary`].
    pub fn exec(&self) -> Result<TestListSummary, CommandError> {
        let mut command = self.cargo_command();
        let output = command.output().map_err(CommandError::Exec)?;

        if !output.status.success() {
            // The process exited with a non-zero code.
            let exit_code = output.status.code();
            let stderr = output.stderr;
            return Err(CommandError::CommandFailed { exit_code, stderr });
        }

        // Try parsing stdout.
        serde_json::from_slice(&output.stdout).map_err(CommandError::Json)
    }

    /// Executes `cargo nextest list --list-type binaries-only` and parses the output into a
    /// [`BinaryListSummary`].
    pub fn exec_binaries_only(&self) -> Result<BinaryListSummary, CommandError> {
        let mut command = self.cargo_command();
        command.arg("--list-type=binaries-only");
        let output = command.output().map_err(CommandError::Exec)?;

        if !output.status.success() {
            // The process exited with a non-zero code.
            let exit_code = output.status.code();
            let stderr = output.stderr;
            return Err(CommandError::CommandFailed { exit_code, stderr });
        }

        // Try parsing stdout.
        serde_json::from_slice(&output.stdout).map_err(CommandError::Json)
    }
}

/// Root element for a serializable list of tests generated by nextest.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct TestListSummary {
    /// Rust metadata used for builds and test runs.
    pub rust_metadata: RustMetadataSummary,

    /// Number of tests (including skipped and ignored) across all binaries.
    pub test_count: usize,

    /// A map of Rust test suites to the test binaries within them, keyed by a unique identifier
    /// for each test suite.
    pub rust_suites: BTreeMap<String, RustTestSuiteSummary>,
}

impl TestListSummary {
    /// Creates a new `TestListSummary` with the given Rust metadata.
    pub fn new(rust_metadata: RustMetadataSummary) -> Self {
        Self {
            rust_metadata,
            test_count: 0,
            rust_suites: BTreeMap::new(),
        }
    }
    /// Parse JSON output from `cargo nextest list --format json`.
    pub fn parse_json(json: impl AsRef<str>) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json.as_ref())
    }
}

/// The platform a binary was built on (useful for cross-compilation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildPlatform {
    /// The target platform.
    Target,

    /// The host platform: the platform the build was performed on.
    Host,
}

impl fmt::Display for BuildPlatform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Target => write!(f, "target"),
            Self::Host => write!(f, "host"),
        }
    }
}

/// A serializable Rust test binary.
///
/// Part of a [`RustTestSuiteSummary`] and [`RustBinaryListSummary`].
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RustTestBinarySummary {
    /// A unique binary ID.
    pub binary_id: String,

    /// The name of the test binary within the package.
    pub binary_name: String,

    /// The unique package ID assigned by Cargo to this test.
    ///
    /// This package ID can be used for lookups in `cargo metadata`.
    pub package_id: String,

    /// The path to the test binary executable.
    pub binary_path: Utf8PathBuf,

    /// Platform for which this binary was built.
    /// (Proc-macro tests are built for the host.)
    pub build_platform: BuildPlatform,
}

/// A serializable suite of test binaries.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BinaryListSummary {
    /// Rust metadata used for builds and test runs.
    pub rust_metadata: RustMetadataSummary,

    /// The list of Rust test binaries (indexed by binary-id).
    pub rust_binaries: BTreeMap<String, RustTestBinarySummary>,
}

/// Rust metadata used for builds and test runs.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RustMetadataSummary {
    /// The target directory for Rust artifacts.
    pub target_directory: Utf8PathBuf,

    /// Base output directories, relative to the target directory.
    pub base_output_directories: BTreeSet<Utf8PathBuf>,

    /// Linked paths, relative to the target directory.
    pub linked_paths: BTreeSet<Utf8PathBuf>,
}

/// A serializable suite of tests within a Rust test binary.
///
/// Part of a [`TestListSummary`].
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RustTestSuiteSummary {
    /// The name of this package in the workspace.
    pub package_name: String,

    /// The binary within the package.
    #[serde(flatten)]
    pub binary: RustTestBinarySummary,

    /// The working directory that tests within this package are run in.
    pub cwd: Utf8PathBuf,

    /// Test case names and other information about them.
    pub testcases: BTreeMap<String, RustTestCaseSummary>,
}

/// Serializable information about an individual test case within a Rust test suite.
///
/// Part of a [`RustTestSuiteSummary`].
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RustTestCaseSummary {
    /// Returns true if this test is marked ignored.
    ///
    /// Ignored tests, if run, are executed with the `--ignored` argument.
    pub ignored: bool,

    /// Whether the test matches the provided test filter.
    ///
    /// Only tests that match the filter are run.
    pub filter_match: FilterMatch,
}

/// An enum describing whether a test matches a filter.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum FilterMatch {
    /// This test matches this filter.
    Matches,

    /// This test does not match this filter.
    Mismatch {
        /// Describes the reason this filter isn't matched.
        reason: MismatchReason,
    },
}

impl FilterMatch {
    /// Returns true if the filter doesn't match.
    pub fn is_match(&self) -> bool {
        matches!(self, FilterMatch::Matches)
    }
}

/// The reason for why a test doesn't match a filter.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum MismatchReason {
    /// This test does not match the run-ignored option in the filter.
    Ignored,

    /// This test does not match the provided string filters.
    String,

    /// This test is in a different partition.
    Partition,
}

impl fmt::Display for MismatchReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MismatchReason::Ignored => write!(f, "does not match the run-ignored option"),
            MismatchReason::String => write!(f, "does not match the provided string filters"),
            MismatchReason::Partition => write!(f, "is in a different partition"),
        }
    }
}
