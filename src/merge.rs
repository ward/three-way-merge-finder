use std::io::prelude::*;
/// Walks through commits, looking for those with (exactly) two parents. Collects parents and
/// the common base.
pub fn find_merges(repo: &git2::Repository, revwalk: git2::Revwalk) -> Vec<ThreeWayMerge> {
    revwalk
        .map(|oid| {
            repo.find_commit(oid.expect("Failed to get Oid"))
                .expect("Failed to turn oid into a commit")
        })
        .filter(|commit| commit.parent_count() == 2)
        .map(|commit| {
            let parent1 = commit
                .parent_id(0)
                .expect("Failed to get id for first parent.");
            let parent2 = commit
                .parent_id(1)
                .expect("Failed to get id for second parent.");
            let base = repo.merge_base(parent1, parent2);
            match base {
                Ok(base) => Some(ThreeWayMerge {
                    o: base,
                    a: parent1,
                    b: parent2,
                    m: commit.id(),
                }),
                Err(e) => {
                    eprintln!(
                        "Could not find base for the two parent commits of {}. Full error: {}",
                        commit.id(),
                        e
                    );
                    None
                }
            }
        })
        .filter(|twm| twm.is_some())
        .map(|twm| twm.unwrap())
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
        changed_files: std::collections::HashSet<String>,
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

        write_files_from_commit_to_disk(folder.join("o"), self.o, repo, &changed_files, "O");
        write_files_from_commit_to_disk(folder.join("a"), self.a, repo, &changed_files, "A");
        write_files_from_commit_to_disk(folder.join("b"), self.b, repo, &changed_files, "B");
        write_files_from_commit_to_disk(folder.join("m"), self.m, repo, &changed_files, "M");
    }
}

/// For a given list of files, locates them in the commit and writes them into the provided
/// folder. The files are placed in subfolders mimicking their folders in the commit.
fn write_files_from_commit_to_disk<P: AsRef<std::path::Path>>(
    folder: P,
    commit: git2::Oid,
    repo: &git2::Repository,
    changed_files: &std::collections::HashSet<String>,
    commit_description: &str,
) {
    let folder = folder.as_ref();
    let commit = repo.find_commit(commit).unwrap();
    let tree = commit.tree().unwrap();
    for file in changed_files {
        let tree_entry = tree.get_path(&std::path::Path::new(&file));
        if tree_entry.is_err() {
            println!(
                "File {} not present in {}. Skipping.",
                &file, commit_description
            );
            continue;
        }
        let tree_entry = tree_entry.unwrap();
        let obj = match tree_entry.to_object(&repo) {
            Ok(obj) => obj,
            Err(err) => {
                eprintln!(
                    "ERR: '{}' when looking for file {} in commit {}. File had tree entry id: {}",
                    err,
                    file,
                    commit.id(),
                    tree_entry.id()
                );
                continue;
            }
        };
        let blob = obj.as_blob().unwrap();
        let fullfilepath = folder.join(file);
        if let Some(filefolder) = fullfilepath.as_path().parent() {
            std::fs::create_dir_all(filefolder).unwrap_or_else(|err| {
                panic!("Failed to create necessary folders to save file from git to disk. File: {:?}, Err: {}",
                    fullfilepath,
                    err);
            });
        }
        let mut writer = std::fs::File::create(&fullfilepath)
            .unwrap_or_else(|_| panic!("Failed to open file for writing {:?}", &fullfilepath));
        writer.write_all(blob.content()).unwrap();
    }
}
