// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Animated moon-phase spinner for long phases (TTY only).
//!
//! When animation is off (pipes, JSON mode, non-interactive stderr) the
//! spinner is inert and only the final `finish` line is printed, so output
//! stays clean for machines while staying alive-feeling for humans.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// One lunar cycle per phase, as is traditional for dreamcatchers.
const MOON: [&str; 8] = ["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘"];

/// A running spinner; inert unless constructed with animation enabled.
pub struct Spinner {
    stop: Arc<AtomicBool>,
    message: Arc<Mutex<String>>,
    handle: Option<JoinHandle<()>>,
}

impl Spinner {
    /// Start a spinner for `msg`, animating only when `animated` is true.
    pub fn start(msg: &str, animated: bool) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let message = Arc::new(Mutex::new(truncate(msg, 64)));
        let handle = animated.then(|| {
            let stop = Arc::clone(&stop);
            let message = Arc::clone(&message);
            thread::spawn(move || {
                let mut frame = 0usize;
                while !stop.load(Ordering::Relaxed) {
                    let msg = message.lock().map(|m| truncate(&m, 64)).unwrap_or_default();
                    eprint!("\r{} {msg}", MOON[frame % MOON.len()]);
                    let _ = std::io::stderr().flush();
                    frame += 1;
                    thread::sleep(Duration::from_millis(120));
                }
                eprint!("\r{}\r", " ".repeat(76));
                let _ = std::io::stderr().flush();
            })
        });
        Self {
            stop,
            message,
            handle,
        }
    }

    /// Update the spinner status line (live while animating).
    pub fn set_message(&self, msg: &str) {
        if let Ok(mut m) = self.message.lock() {
            *m = msg.to_string();
        }
    }

    /// Stop the spinner and replace it with a final line (always printed).
    pub fn finish(self, line: &str) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle {
            let _ = h.join();
        }
        if !line.is_empty() {
            eprintln!("{line}");
        }
    }
}

/// Clamp a status message to `max` chars, ellipsising on the right.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inert_spinner_still_prints_finish_line() {
        let sp = Spinner::start("working", false);
        sp.set_message("still working");
        sp.finish("done line");
    }

    #[test]
    fn animated_spinner_starts_and_stops() {
        let sp = Spinner::start("working", true);
        thread::sleep(Duration::from_millis(200));
        sp.finish("");
    }

    #[test]
    fn truncate_boundaries() {
        assert_eq!(truncate("short", 10), "short");
        let long = "x".repeat(20);
        assert_eq!(truncate(&long, 10).chars().count(), 10);
        assert!(truncate(&long, 10).ends_with('…'));
    }
}
