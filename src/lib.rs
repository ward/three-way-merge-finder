pub fn create_revwalk(repo: &git2::Repository) -> Result<git2::Revwalk, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    // Pushing marks a commit to start traversal from
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
    Ok(revwalk)
}

pub mod publish {
    pub fn print_csv_of_merges(repo: &git2::Repository, revwalk: git2::Revwalk) {
        let merges = super::merge::find_merges(repo, revwalk);
        println!("O,A,B,M");
        for merge in merges {
            println!("{}", merge.to_csv_line());
        }
    }

    /// Finds the merges of a given git repository, dumps the changed files for each of them into
    /// the provided folder. Final structure of that folder will be:
    /// folder/mergehash/mergepart/path/to/file
    ///
    /// Folder needs to be empty, may or may not exist.
    pub fn folder_dump<P: AsRef<std::path::Path>>(
        folder: P,
        repo: &git2::Repository,
        revwalk: git2::Revwalk,
    ) {
        let folder = folder.as_ref();
        // Create folder if needed and check it is empty
        std::fs::create_dir_all(&folder).expect("Could not create output-folder");
        let mut dir_contents = std::fs::read_dir(&folder).expect("Could not read output-folder");
        if dir_contents.next().is_some() {
            panic!("Specified output-folder is not empty. Aborting.");
        }

        let merges = super::merge::find_merges(repo, revwalk);

        // Create merge-hash folder and its o, a, b, and m subfolders.
        for merge in merges {
            let files = merge.files_to_consider(&repo);
            let merge_path = folder.join(merge.m.to_string());
            merge.write_files_to_disk(&merge_path, files, &repo);
        }
        // TODO? Create a csv file of all merges in the folder
        // TODO? Place detailed diff "overview" in a text file there
    }
}

mod merge {
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
                let base = repo
                    .merge_base(parent1, parent2)
                    .expect("Could not find base for the two parent commits");
                ThreeWayMerge {
                    o: base,
                    a: parent1,
                    b: parent2,
                    m: commit.id(),
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
        pub fn files_to_consider(
            &self,
            repo: &git2::Repository,
        ) -> std::collections::HashSet<String> {
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
                    "Failed to find file {} in {}. Skipping.",
                    &file, commit_description
                );
                continue;
            }
            let tree_entry = tree_entry.unwrap();
            let obj = tree_entry.to_object(&repo).unwrap();
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
}

pub mod debugging {
    pub fn diff_walk(repo: &git2::Repository, revwalk: git2::Revwalk) {
        let mut diffoptions = git2::DiffOptions::new();
        diffoptions.minimal(true).ignore_whitespace(true);
        for oid in revwalk {
            let oid = oid.expect("Could not get oid");
            let commit = repo
                .find_commit(oid)
                .expect("Failed to turn oid into a commit");
            let ctree = commit
                .tree()
                .expect("Should be able to find tree for commit");
            println!("Handling commit: {:?}", commit);
            for parent in commit.parents() {
                let ptree = parent
                    .tree()
                    .expect("Should be able to find tree for parent commit");
                let diff = repo
                    .diff_tree_to_tree(Some(&ptree), Some(&ctree), Some(&mut diffoptions))
                    .expect("Should be able to diff parent to commit");
                println!(
                    "Diffing with parent {:?}, stats: {:#?}",
                    parent,
                    diff.stats()
                );
                for delta in diff.deltas() {
                    println!(
                        "{:?}, {:?}, {:?}",
                        delta.status(),
                        delta.old_file(),
                        delta.new_file()
                    );
                }
            }
            println!("-------------------------------");
        }
    }

    fn _commit_printing(repo: git2::Repository, revwalk: git2::Revwalk) {
        for oid in revwalk {
            let oid = oid.expect("Expected to get an object identity (oid), but it failed.");
            let commit = repo
                .find_commit(oid)
                .expect("Failed to turn oid into a commit.");
            println!(
                "commit {:?} has {} parents",
                commit.id(),
                commit.parent_count()
            );
            for (ctr, parent) in commit.parent_ids().enumerate() {
                println!("    Parent nr {}: {}", ctr + 1, parent);
            }
            // merge_base_many expects a fixed size array, not sure if I can easily go from an iterator
            // to that. Instead just limit it to the merges with exactly two parents. This is the
            // vaaaast majority anyway. Also the only ones I really consider.
            if commit.parent_count() == 2 {
                let parent1 = commit
                    .parent_id(0)
                    .expect("Failed to get id for first parent.");
                let parent2 = commit
                    .parent_id(1)
                    .expect("Failed to get id for second parent.");
                match repo.merge_base(parent1, parent2) {
                    Ok(oid) => println!("Base at {}", oid),
                    Err(e) => panic!("Could not find base for these parent commits. {}", e),
                }
            }
        }
    }
}
