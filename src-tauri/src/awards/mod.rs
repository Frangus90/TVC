//! Awards data: fetch ceremony pages from Wikipedia (MediaWiki API), parse them
//! into structured categories/nominees/winners, and score the user's predictions.
//!
//! This module is the data + logic foundation. Persistence (SQLite upserts) and
//! the Tauri commands / sidebar UI are layered on in later phases; until then some
//! items are only exercised by tests, hence the module-wide dead_code allowance.
#![allow(dead_code)]

pub mod db;
pub mod models;
pub mod scoring;
pub mod source;
pub mod sync;
pub mod wikipedia;
