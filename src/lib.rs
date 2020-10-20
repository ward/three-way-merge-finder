//! This crate works on git repositories. Currently there are two goals:
//!
//! * Find the four commits that form a three way merge (Origin, A-side, B-side, Merge commit)
//! * Find the fix for a buggy commit.

#[macro_use]
extern crate lazy_static;

/// Creates a toplogical revwalk over a repository, starting at HEAD.
pub fn create_revwalk(repo: &git2::Repository) -> Result<git2::Revwalk, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    // Pushing marks a commit to start traversal from
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
    Ok(revwalk)
}

pub mod publish;

mod merge;

pub mod debugging;

pub mod find_bug_fix;
