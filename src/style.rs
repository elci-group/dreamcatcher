// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Terminal styling: ANSI colours and emoji, gated on TTY and `NO_COLOR`.

use form3::ansi::Color as AnsiColor;
use form3::compat::Colorize;
use form3::term::ColorSupport;
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
    if !enabled() {
        return text.to_string();
    }
    let styled = match color {
        Color::Bold => text.bold(),
        Color::Dim => text.dimmed(),
        Color::Cyan => text.color(AnsiColor::Cyan),
        Color::Green => text.color(AnsiColor::Green),
        Color::Yellow => text.color(AnsiColor::Yellow),
        Color::Red => text.color(AnsiColor::Red),
        Color::Magenta => text.color(AnsiColor::Magenta),
    };
    styled.with_color_support(ColorSupport::Standard16).to_string()
}
