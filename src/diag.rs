//! Findings produced by resolution and linting.

/// Ordering is meaningful. The worst severity in a run is a `max` over the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Reported, but the build proceeds unless `--strict` is set.
    Warn,
    /// The plan is not executable.
    Deny,
}

/// A single finding, ready to render.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Diag {
    /// Stable name such as `fold-before-inject`, never reused for another meaning.
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    /// What to do about it, one `help:` line per line.
    pub help: Option<String>,
}

impl Diag {
    /// Builds a finding that makes the plan unexecutable.
    pub fn deny(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Deny,
            message: message.into(),
            help: None,
        }
    }

    /// Builds a finding that is reported but does not stop a build.
    pub fn warn(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Warn,
            message: message.into(),
            help: None,
        }
    }

    #[must_use]
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Orders findings by severity, then code, then message.
///
/// Findings arrive from several passes in whatever order those finish, so this is what
/// makes two runs over the same manifest print identically.
pub fn sort(diags: &mut [Diag]) {
    diags.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.code.cmp(b.code))
            .then_with(|| a.message.cmp(&b.message))
    });
}

pub fn worst(diags: &[Diag]) -> Option<Severity> {
    diags.iter().map(|d| d.severity).max()
}

pub fn count(diags: &[Diag], severity: Severity) -> usize {
    diags.iter().filter(|d| d.severity == severity).count()
}
