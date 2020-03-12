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
