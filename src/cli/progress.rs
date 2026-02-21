//! Pretty console output helpers for the CLI.
//!
//! Provides progress bars, spinners, and styled terminal messages using
//! [`indicatif`] and [`console`].

use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

// ── Symbols ──────────────────────────────────────────────────────────────────

pub const TICK_OK: &str = "✓";
pub const TICK_WARN: &str = "⚠";
pub const TICK_ERR: &str = "✗";
pub const TICK_INFO: &str = "›";

// ── Banner ────────────────────────────────────────────────────────────────────

/// Print the CLI banner (shown once at startup).
pub fn print_banner() {
    let term = Term::stderr();
    let width = term.size().1 as usize;
    let divider = "─".repeat(width.min(60));

    eprintln!(
        "\n{} {}\n{}",
        style("city3dstac").bold().cyan(),
        style("STAC metadata generator for 3D city models").dim(),
        style(&divider).dim(),
    );
}

// ── Single-item spinner ───────────────────────────────────────────────────────

/// Create and start a spinner for a single-item operation.
///
/// Call [`finish_spinner_ok`] / [`finish_spinner_err`] when done.
pub fn create_spinner(message: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.into());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn finish_spinner_ok(pb: ProgressBar, message: impl Into<String>) {
    pb.finish_and_clear();
    eprintln!("{} {}", style(TICK_OK).bold().green(), message.into());
}

pub fn finish_spinner_err(pb: ProgressBar, message: impl Into<String>) {
    pb.finish_and_clear();
    eprintln!("{} {}", style(TICK_ERR).bold().red(), message.into());
}

// ── Multi-item progress bar ───────────────────────────────────────────────────

/// Create a determinate progress bar for batch operations.
pub fn create_progress_bar(total: u64, message: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{msg}\n{spinner:.cyan} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏  ")
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.into());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

// ── Pretty messages ───────────────────────────────────────────────────────────

pub fn print_success(message: impl AsRef<str>) {
    eprintln!("{} {}", style(TICK_OK).bold().green(), message.as_ref());
}

pub fn print_warning(message: impl AsRef<str>) {
    eprintln!("{} {}", style(TICK_WARN).bold().yellow(), message.as_ref());
}

pub fn print_error(message: impl AsRef<str>) {
    eprintln!("{} {}", style(TICK_ERR).bold().red(), message.as_ref());
}

pub fn print_info(message: impl AsRef<str>) {
    eprintln!("{} {}", style(TICK_INFO).bold().blue(), message.as_ref());
}

/// Print a section header with a subtle divider.
pub fn print_section(label: impl AsRef<str>) {
    eprintln!("\n{}", style(label.as_ref()).bold().underlined());
}

// ── Summary box ───────────────────────────────────────────────────────────────

/// A simple key/value summary printed at the end of a command.
pub struct Summary {
    rows: Vec<(String, String)>,
}

impl Summary {
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    pub fn add(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.rows.push((key.into(), value.into()));
        self
    }

    pub fn print(self) {
        let key_width = self.rows.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
        eprintln!();
        for (key, value) in self.rows {
            eprintln!(
                "  {:<width$}  {}",
                style(key).dim(),
                style(value).bold(),
                width = key_width,
            );
        }
        eprintln!();
    }
}

impl Default for Summary {
    fn default() -> Self {
        Self::new()
    }
}
