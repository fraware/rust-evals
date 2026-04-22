//! Container engine abstraction.
//!
//! The trait is minimal on purpose: prepare an image, resolve its digest,
//! execute one command in a fresh container at a given workdir under a
//! given resource ceiling, and return the captured outcome. Real container
//! back-ends (Docker via `bollard`, podman, etc.) implement it by
//! delegating to the daemon; the [`LocalProcessEngine`] implements it by
//! spawning a host subprocess. The latter is what CI uses to exercise the
//! full L0/L1 pipeline on machines without Docker.
//!
//! # Contract
//!
//! - `prepare_image` must be idempotent.
//! - `exec` must never leak resources on failure (containers are disposed
//!   regardless of exit status).
//! - `exec` must enforce `limits.wall_timeout` when it is `Some`; a timed
//!   out run returns `Ok(ExecOutcome { timed_out: true, .. })`, not an
//!   `Err`, so the pipeline can record the event and continue.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Resource limits for a single run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    /// CPU quota string recognized by the container engine (for example `"2"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_limit: Option<String>,
    /// Memory quota string recognized by the container engine (for example `"8g"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<String>,
    /// Wall-clock timeout for the whole run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_timeout: Option<Duration>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_limit: None,
            memory_limit: None,
            wall_timeout: Some(Duration::from_secs(30 * 60)),
        }
    }
}

/// Environment variable in a form cheap to serialize and easy to construct
/// from a `HashMap` or an iterator of pairs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvVar {
    /// Variable name.
    pub name: String,
    /// Variable value. Not treated as a secret; callers must scrub before
    /// attaching the spec to an evidence bundle.
    pub value: String,
}

/// Declarative specification of one container command execution.
///
/// The spec is deliberately value-typed (no references); the pipeline
/// writes a JSON serialization into the evidence bundle so reviewers can
/// replay the exact command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecSpec {
    /// Resolved image reference (may be `sha256:<digest>` or an engine-local name).
    pub image: String,
    /// Working directory *inside the container* (or on the host for
    /// [`LocalProcessEngine`]).
    pub workdir: PathBuf,
    /// Command and arguments. Never passed to a shell.
    pub command: Vec<String>,
    /// Additional environment variables to set. Host variables are *not*
    /// inherited by default; see [`LocalProcessEngine`].
    #[serde(default)]
    pub env: Vec<EnvVar>,
    /// Resource limits.
    pub limits: ResourceLimits,
}

/// Outcome of a single in-container command execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecOutcome {
    /// Exit code, if the process terminated normally.
    pub exit_code: Option<i32>,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
    /// Observed wall time in seconds. Nondeterministic; excluded from any
    /// content-addressed hash the pipeline computes.
    pub wall_time_secs: f64,
    /// Whether the process was killed by the harness wall timeout.
    pub timed_out: bool,
}

/// Trait implemented by container engines.
pub trait ContainerEngine: Send + Sync + std::fmt::Debug {
    /// Pull or verify an image. Returns the resolved image digest as
    /// `sha256:<64-hex>`.
    fn prepare_image(&self, image: &str) -> Result<String, ContainerEngineError>;

    /// Execute `spec` in a fresh container and return the captured outcome.
    fn exec(&self, spec: &ExecSpec) -> Result<ExecOutcome, ContainerEngineError>;
}

/// Errors produced by the container engine.
#[derive(Debug, Error)]
pub enum ContainerEngineError {
    /// No backend is available. Compile with `--features docker` to enable
    /// the Docker backend.
    #[error(
        "no container backend is available; rebuild eval-ladder-runner with --features docker"
    )]
    NoBackendAvailable,
    /// The backend rejected the image reference.
    #[error("image not found: {0}")]
    ImageNotFound(String),
    /// The backend failed to start the container.
    #[error("container start failed: {0}")]
    StartFailed(String),
    /// Command execution failed at the engine level (not a non-zero exit).
    #[error("exec failed: {0}")]
    ExecFailed(String),
    /// Working directory does not exist on the host (for [`LocalProcessEngine`]).
    #[error("workdir does not exist: {0}")]
    WorkdirMissing(PathBuf),
    /// Command vector was empty.
    #[error("empty command vector")]
    EmptyCommand,
}

/// In-memory stub engine used in tests that do not need real execution.
///
/// Every `exec` call returns a canned successful outcome with empty stdout/stderr.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopEngine;

impl ContainerEngine for NoopEngine {
    fn prepare_image(&self, _image: &str) -> Result<String, ContainerEngineError> {
        Ok("sha256:0000000000000000000000000000000000000000000000000000000000000000".to_owned())
    }

    fn exec(&self, _spec: &ExecSpec) -> Result<ExecOutcome, ContainerEngineError> {
        Ok(ExecOutcome {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            wall_time_secs: 0.0,
            timed_out: false,
        })
    }
}

/// Host-process backend: spawns the command directly in the requested
/// workdir and captures stdout/stderr.
///
/// This is **not** an isolated container; it exists so that CI and the
/// Milestone C fixture pipeline can run end-to-end on machines without
/// Docker.
///
/// # Environment semantics
///
/// Host `PATH` is inherited so commands like `cargo --version` resolve.
/// All other environment variables are dropped unless listed in
/// [`ExecSpec::env`]. This is a deliberate divergence from the default
/// Docker behaviour (which would also drop `PATH`) chosen because CI
/// scorers are invoked by name from `$PATH`. If you need a more isolated
/// environment, wrap `LocalProcessEngine` in a restricted shell.
///
/// # Wall timeout
///
/// Enforced by polling the child every 50 ms. On timeout the child is
/// killed (`kill`) and the outcome reports `timed_out = true`.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalProcessEngine;

impl ContainerEngine for LocalProcessEngine {
    fn prepare_image(&self, image: &str) -> Result<String, ContainerEngineError> {
        // For local execution we do not resolve a real digest; return a
        // stable sentinel that identifies the local engine. The pipeline
        // records this verbatim in `container_metadata.json` so reviewers
        // can tell local runs apart from containerized ones.
        let _ = image;
        Ok("sha256:\
            0000000000000000000000000000000000000000000000000000000000000000"
            .to_owned())
    }

    fn exec(&self, spec: &ExecSpec) -> Result<ExecOutcome, ContainerEngineError> {
        if spec.command.is_empty() {
            return Err(ContainerEngineError::EmptyCommand);
        }
        if !spec.workdir.exists() {
            return Err(ContainerEngineError::WorkdirMissing(spec.workdir.clone()));
        }

        let (program, args) = spec.command.split_first().expect("non-empty command");

        let mut cmd = std::process::Command::new(program);
        cmd.args(args)
            .current_dir(&spec.workdir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .env_clear();

        // PATH must survive env_clear so named commands still resolve.
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }
        #[cfg(windows)]
        {
            if let Ok(systemroot) = std::env::var("SystemRoot") {
                cmd.env("SystemRoot", systemroot);
            }
            if let Ok(pathext) = std::env::var("PATHEXT") {
                cmd.env("PATHEXT", pathext);
            }
        }
        for EnvVar { name, value } in &spec.env {
            cmd.env(name, value);
        }

        let start = Instant::now();
        let child = cmd
            .spawn()
            .map_err(|e| ContainerEngineError::ExecFailed(format!("spawn {program}: {e}")))?;

        let outcome = wait_with_timeout(child, spec.limits.wall_timeout, start)?;
        Ok(outcome)
    }
}

fn wait_with_timeout(
    mut child: std::process::Child,
    wall_timeout: Option<Duration>,
    start: Instant,
) -> Result<ExecOutcome, ContainerEngineError> {
    let poll_interval = Duration::from_millis(50);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let (stdout, stderr) = read_stdio(&mut child);
                return Ok(ExecOutcome {
                    exit_code: status.code(),
                    stdout,
                    stderr,
                    wall_time_secs: start.elapsed().as_secs_f64(),
                    timed_out: false,
                });
            }
            Ok(None) => {
                if let Some(limit) = wall_timeout {
                    if start.elapsed() >= limit {
                        let _ = child.kill();
                        let _ = child.wait();
                        let (stdout, stderr) = read_stdio(&mut child);
                        return Ok(ExecOutcome {
                            exit_code: None,
                            stdout,
                            stderr,
                            wall_time_secs: start.elapsed().as_secs_f64(),
                            timed_out: true,
                        });
                    }
                }
                std::thread::sleep(poll_interval);
            }
            Err(e) => {
                return Err(ContainerEngineError::ExecFailed(format!("wait: {e}")));
            }
        }
    }
}

fn read_stdio(child: &mut std::process::Child) -> (String, String) {
    use std::io::Read;
    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_string(&mut stdout);
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_string(&mut stderr);
    }
    (stdout, stderr)
}

/// Convenience constructor for [`ExecSpec`] used throughout the pipeline
/// and in tests.
impl ExecSpec {
    /// Build an [`ExecSpec`] from its canonical parts.
    #[must_use]
    pub fn new(
        image: impl Into<String>,
        workdir: impl AsRef<Path>,
        command: Vec<String>,
        env: Vec<EnvVar>,
        limits: ResourceLimits,
    ) -> Self {
        Self {
            image: image.into(),
            workdir: workdir.as_ref().to_path_buf(),
            command,
            env,
            limits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn noop_engine_returns_empty_pass() {
        let e = NoopEngine;
        let spec = ExecSpec::new(
            "local:stub",
            std::env::temp_dir(),
            vec!["noop".into()],
            Vec::new(),
            ResourceLimits::default(),
        );
        let out = e.exec(&spec).unwrap();
        assert_eq!(out.exit_code, Some(0));
        assert!(out.stdout.is_empty());
    }

    #[test]
    fn local_engine_rejects_empty_command() {
        let dir = tempdir().unwrap();
        let e = LocalProcessEngine;
        let spec = ExecSpec::new(
            "local:stub",
            dir.path(),
            Vec::new(),
            Vec::new(),
            ResourceLimits::default(),
        );
        let err = e.exec(&spec).unwrap_err();
        assert!(matches!(err, ContainerEngineError::EmptyCommand));
    }

    #[test]
    fn local_engine_rejects_missing_workdir() {
        let e = LocalProcessEngine;
        let spec = ExecSpec::new(
            "local:stub",
            Path::new("/does/not/exist/hopefully"),
            vec!["cargo".into(), "--version".into()],
            Vec::new(),
            ResourceLimits::default(),
        );
        let err = e.exec(&spec).unwrap_err();
        assert!(matches!(err, ContainerEngineError::WorkdirMissing(_)));
    }

    #[test]
    fn local_engine_runs_cargo_version() {
        let dir = tempdir().unwrap();
        let e = LocalProcessEngine;
        let spec = ExecSpec::new(
            "local:stub",
            dir.path(),
            vec!["cargo".into(), "--version".into()],
            Vec::new(),
            ResourceLimits {
                wall_timeout: Some(Duration::from_secs(30)),
                ..ResourceLimits::default()
            },
        );
        let out = e.exec(&spec).unwrap();
        assert_eq!(out.exit_code, Some(0));
        assert!(out.stdout.to_lowercase().starts_with("cargo "));
        assert!(!out.timed_out);
    }
}
