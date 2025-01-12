/* Copyright (C) 2024  AlphaKeks <alphakeks@dawn.sh>
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this repository.  If not, see <https://www.gnu.org/licenses/>.
 */

#![feature(array_try_from_fn)]
#![feature(assert_matches)]
#![feature(decl_macro)]
#![feature(iter_chain)]
#![feature(trait_alias)]
#![feature(try_blocks)]

#[macro_use]
extern crate derive_more;

#[allow(unused_imports)]
#[macro_use(trace, debug, debug_span, info, info_span, warn, error)]
extern crate tracing;

#[macro_use(pin_project)]
extern crate pin_project;

#[macro_use(select)]
extern crate tokio;

#[macro_use]
mod macros;

pub mod config;
pub use config::Config;

pub mod context;
pub use context::Context;

pub mod database;

pub mod plugin;
pub mod access_keys;
pub mod users;
pub mod servers;
pub mod players;
pub mod maps;
pub mod jumpstats;
pub mod records;
pub mod bans;
pub mod points;

pub mod email;
pub mod events;
pub mod git;
pub mod mode;
pub mod pagination;
pub mod steam;
pub mod styles;
pub mod time;

mod python;

mod fmt;
mod num;
