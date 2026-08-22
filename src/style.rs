// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Terminal styling: ANSI colours and emoji, gated on TTY and `NO_COLOR`.

use std::sync::OnceLock;

/// ANSI colour/weight codes used by the output layer.
#[derive(Debug, Clone, Copy)]
pub enum Color {
    /// Bold default foreground.
    Bold,
    /// Dimmed/grey text.
    Dim,
    /// Cyan.
    Cyan,
    /// Green.
    Green,
    /// Yellow.
    Yellow,
    /// Red.
    Red,
    /// Magenta.
    Magenta,
}

impl Color {
    fn code(self) -> &'static str {
        match self {
            Color::Bold => "\x1b[1m",
            Color::Dim => "\x1b[2m",
            Color::Cyan => "\x1b[36m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Red => "\x1b[31m",
            Color::Magenta => "\x1b[35m",
        }
    }
}

static ENABLED: OnceLock<bool> = OnceLock::new();

/// Initialise the styling layer. Call once at CLI startup.
pub fn init(enabled: bool) {
    let _ = ENABLED.set(enabled);
}

/// True when styling was enabled at startup.
pub fn enabled() -> bool {
    *ENABLED.get().unwrap_or(&false)
}

/// Styling should be on for interactive terminals without `NO_COLOR`.
pub fn should_enable(is_tty: bool) -> bool {
    is_tty && std::env::var_os("NO_COLOR").is_none()
}

/// Wrap `text` in the given colour when styling is enabled.
pub fn paint(color: Color, text: &str) -> String {
    if enabled() {
        format!("{}{}\x1b[0m", color.code(), text)
    } else {
        text.to_string()
    }
}
