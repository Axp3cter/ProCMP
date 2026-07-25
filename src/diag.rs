//! Findings produced by resolution and checking.

/// Ordering is meaningful: the worst severity in a run is a `max` over the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warn,
    Deny,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Diag {
    /// A stable name such as `fold-before-inject`, never reused for another meaning.
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub help: Option<String>,
}

impl Diag {
    pub fn deny(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, Severity::Deny, message)
    }

    pub fn warn(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, Severity::Warn, message)
    }

    fn new(code: &'static str, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            code,
            severity,
            message: message.into(),
            help: None,
        }
    }

    /// Called twice, both lines survive.
    #[must_use]
    pub fn help(mut self, help: impl Into<String>) -> Self {
        let line = help.into();
        self.help = Some(match self.help {
            Some(existing) => format!("{existing}\n{line}"),
            None => line,
        });
        self
    }
}

/// Severity, then code, then message, so two runs print identically.
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

/// Errors and warnings, from one walk.
pub fn tally(diags: &[Diag]) -> (usize, usize) {
    diags
        .iter()
        .fold((0, 0), |(deny, warn), diag| match diag.severity {
            Severity::Deny => (deny + 1, warn),
            Severity::Warn => (deny, warn + 1),
        })
}
