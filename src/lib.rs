//! This crate works on git repositories. Currently there are two goals:
//!
//! * Find the four commits that form a three way merge (Origin, A-side, B-side, Merge commit)
//! * Find the fix for a buggy commit.

#[macro_use]
extern crate lazy_static;

pub mod publish;

mod merge;

pub mod debugging;

pub mod find_bug_fix;

pub mod git_utils;

mod relative_files;
