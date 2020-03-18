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
            println!("{},{},{},{}", merge.o, merge.a, merge.b, merge.m);
            // super::merge::compare_commits(&repo, &merge);
        }
    }

    pub fn folder_dump(folder: &str, clean_output: bool) {
        // 1. Create folder if needed
        // 2. Clean folder if asked for
        // 3. Create a csv file of all merges in the folder
        // 4. Create folder for every merge commit (just use hash as name)
        // 5. Create O, A, B, and M folders in merge commit folder
        // 6. Place detailed diff "overview" in a text file there?
        // 7. Place files in O, A, B, and M folders (which exactly?)
    }
}

pub mod merge {
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
