// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! The `.dream` input format: model and parser.

pub mod model;
pub mod parse;

pub use model::{
    Dream, DreamId, Effort, Evidence, Intent, IntentKind, Meta, Priority, Risk, Subject,
};
pub use parse::{parse_dream, ParseError};
