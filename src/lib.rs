// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
#![forbid(unsafe_code)]

//! `form3`: dependency-free ANSI terminal styling, tables, and animation
//! primitives. No third-party crates, no proc macros — just the SGR
//! sequences ([`ansi`]), a chainable styling API compatible with the
//! common `Colorize` vocabulary ([`compat`]), Unicode-aware table
//! rendering ([`table`]), and frame-based animation ([`anim`]).
//!
//! Color is opt-in per stream: [`term::TermInfo::detect`] reads the
//! environment-wide `NO_COLOR`/`TERM=dumb` signals, and callers layer
//! their own per-stream TTY check on top via
//! [`compat::StyledText::with_color_support`].
//!
//! For model/agent workflows, enable the `agent` feature and use the
//! typed API in [`agent`].

pub mod anim;
pub mod ansi;
pub mod compat;
pub mod table;
pub mod term;

#[cfg(feature = "agent")]
pub mod agent;

mod width;

pub use anim::{Aura, GrowthTrack, IsopodCrawl, Mode};
pub use compat::{Colorize, StyledText};
pub use term::{ColorSupport, TermInfo};
