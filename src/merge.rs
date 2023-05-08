//! This module is used to find three way merges

use crate::git_utils;
use std::collections::HashSet;

/// Walks through commits, looking for those with (exactly) two parents. Collects parents and
/// the common base.
pub fn find_merges(
    repo: &git2::Repository,
    revwalk: git2::Revwalk,
    before: Option<i64>,
) -> Vec<ThreeWayMerge> {
    revwalk
        .map(|oid| {
            repo.find_commit(oid.expect("Failed to get Oid"))
                .expect("Failed to turn oid into a commit")
        })
        .filter(|commit| commit.parent_count() == 2)
        .filter(|commit| {
            if let Some(before) = before {
                commit.time().seconds() < before
            } else {
                true
            }
        })
        // filter_map is map + flatten. Filters out None and unwraps Some
        .filter_map(|commit| {
            match ThreeWayMerge::new(repo, &commit) {
                Ok(twm) => Some(twm),
                Err(e) => {
                    eprintln!(
                        "Failed to find either parent commits or their common base for {}. Full error: {}",
                        commit.id(),
                        e
                    );
                    None
                }
            }
        })
        .collect()
}

/// Represents the four parts of a merge by storing the Oid of the merge commit, its parent
/// commits, and the original base commit.
pub struct ThreeWayMerge {
    /// The original base commit
    pub o: git2::Oid,
    /// One parent
    pub a: git2::Oid,
    /// Another parent
    pub b: git2::Oid,
    /// The merge commit
    pub m: git2::Oid,
}

impl ThreeWayMerge {
    // Create new ThreeWayMerge based on a valid merge commit.
    fn new(repo: &git2::Repository, commit: &git2::Commit) -> Result<ThreeWayMerge, git2::Error> {
        // Parent order is deterministic and saved as part of the merge commit. Subsequent runs
        // will thus give the same parents for each position.
        let parent1 = commit.parent_id(0)?;
        let parent2 = commit.parent_id(1)?;
        let base = repo.merge_base(parent1, parent2)?;
        Ok(ThreeWayMerge {
            o: base,
            a: parent1,
            b: parent2,
            m: commit.id(),
        })
    }

    /// Return a comma separated line of the four commits that form a three way merge. Order:
    /// O,A,B,M.
    pub fn to_csv_line(&self) -> String {
        format!(
            "{o},{a},{b},{m}",
            o = self.o,
            a = self.a,
            b = self.b,
            m = self.m
        )
    }

    pub fn from_oid_str(
        o_str: &str,
        a_str: &str,
        b_str: &str,
        m_str: &str,
    ) -> Result<Self, git2::Error> {
        let o = git2::Oid::from_str(o_str)?;
        let a = git2::Oid::from_str(a_str)?;
        let b = git2::Oid::from_str(b_str)?;
        let m = git2::Oid::from_str(m_str)?;
        Ok(Self { o, a, b, m })
    }

    /// Analyse the merge diffs to decide which files have been modified and are thus
    /// interesting.
    ///
    /// Currently this only considers O to M, which may miss some changed behaviour
    /// disappearing again. TODO
    pub fn files_to_consider(&self, repo: &git2::Repository) -> std::collections::HashSet<String> {
        let mut diffoptions = git2::DiffOptions::new();
        diffoptions.minimal(true).ignore_whitespace(true);
        let o = repo.find_commit(self.o).expect("Failed to find O commit");
        let otree = o.tree().expect("Failed to find tree for commit O");
        let m = repo.find_commit(self.m).expect("Failed to find M commit");
        let mtree = m.tree().expect("Failed to find tree for commit M");
        let diff = repo
            .diff_tree_to_tree(Some(&otree), Some(&mtree), Some(&mut diffoptions))
            .expect("Should be able to diff O to M");
        let mut paths = std::collections::HashSet::new();
        for delta in diff.deltas() {
            paths.insert(
                delta
                    .old_file()
                    .path()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
            );
            paths.insert(
                delta
                    .new_file()
                    .path()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        paths
    }

    /// For a given list of files, locates them in each part of the ThreeWayMerge. Places them
    /// in o, a, b, or m folders which are created as subfolders of the provided folder.
    pub fn write_files_to_disk<P: AsRef<std::path::Path>>(
        &self,
        folder: P,
        files: std::collections::HashSet<String>,
        repo: &git2::Repository,
    ) {
        let folder = folder.as_ref();
        let paths = [
            folder.join("o"),
            folder.join("a"),
            folder.join("b"),
            folder.join("m"),
        ];
        for path in &paths {
            std::fs::create_dir_all(path).expect("Could not create folder");
        }

        git_utils::write_files_from_commit_to_disk(folder.join("o"), self.o, repo, &files, "O");
        git_utils::write_files_from_commit_to_disk(folder.join("a"), self.a, repo, &files, "A");
        git_utils::write_files_from_commit_to_disk(folder.join("b"), self.b, repo, &files, "B");
        git_utils::write_files_from_commit_to_disk(folder.join("m"), self.m, repo, &files, "M");
    }

    /// For O, A, B, and M, writes all the files in each version to disk. In other words, a file
    /// does not need to be present in all four parts, let alone needing to have a change.
    pub fn write_all_files_to_disk<P: AsRef<std::path::Path>>(
        &self,
        folder: P,
        repo: &git2::Repository,
    ) {
        let folder = folder.as_ref();
        let paths = [
            folder.join("o"),
            folder.join("a"),
            folder.join("b"),
            folder.join("m"),
        ];
        for path in &paths {
            std::fs::create_dir_all(path).expect("Could not create folder");
        }

        // Create a list of all files for each version
        let commit = repo.find_commit(self.o).unwrap();
        let o_paths = git_utils::get_all_paths(&commit.tree().unwrap(), "", repo);
        let commit = repo.find_commit(self.a).unwrap();
        let a_paths = git_utils::get_all_paths(&commit.tree().unwrap(), "", repo);
        let commit = repo.find_commit(self.b).unwrap();
        let b_paths = git_utils::get_all_paths(&commit.tree().unwrap(), "", repo);
        let commit = repo.find_commit(self.m).unwrap();
        let m_paths = git_utils::get_all_paths(&commit.tree().unwrap(), "", repo);

        git_utils::write_files_from_commit_to_disk(folder.join("o"), self.o, repo, &o_paths, "O");
        git_utils::write_files_from_commit_to_disk(folder.join("a"), self.a, repo, &a_paths, "A");
        git_utils::write_files_from_commit_to_disk(folder.join("b"), self.b, repo, &b_paths, "B");
        git_utils::write_files_from_commit_to_disk(folder.join("m"), self.m, repo, &m_paths, "M");
    }

    /// Returns epoch seconds for the merge commit of the ThreeWayMerge. Timezone information is
    /// discarded.
    pub fn time(&self, repo: &git2::Repository) -> i64 {
        repo.find_commit(self.m)
            .expect("Failed to find merge commit")
            .time()
            .seconds()
    }

    /// Check whether O is a different commit than A or B. If it is the same as either, then we're
    /// not *really* working with a twm, but more the joining of a PR to an unchanged master
    /// branch. In other words, no changes on the other side.
    pub fn has_distinct_o(&self) -> bool {
        self.o != self.a && self.o != self.b
    }

    pub fn a_b_change_same_file(&self, repo: &git2::Repository, only_extensions: &[&str]) -> bool {
        crate::git_utils::changed_same_file(
            repo,
            &self.o,
            &self.a,
            &self.o,
            &self.b,
            only_extensions,
        )
    }

    /// Returns a list of files that were changed in O→A AND in O→B
    pub fn files_changed_in_both_branches(&self, repo: &git2::Repository) -> HashSet<String> {
        let o_to_a = git_utils::changed_filenames(repo, &self.o, &self.a);
        let o_to_b = git_utils::changed_filenames(repo, &self.o, &self.b);
        o_to_a
            .intersection(&o_to_b)
            .map(|filename| filename.to_owned())
            .collect()
    }
}
