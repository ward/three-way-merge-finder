fn main() {
    // let repo = match git2::Repository::open("/Users/wardmuylaert/prog/fake-js-project/") {
    let repo = match git2::Repository::open("/Users/wardmuylaert/prog/gumtree/") {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open: {}", e),
    };
    let mut revwalk = match repo.revwalk() {
        Ok(revwalk) => revwalk,
        Err(e) => panic!("Could not get revwalk to walk over commits: {}", e),
    };
    // Pushing marks a commit to start traversal from
    revwalk
        .push_head()
        .expect("Failed to set HEAD as the revwalk starting point");
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL);
    let merges = find_merges(&repo, revwalk);
    for merge in merges {
        println!("{},{},{},{}", merge.o, merge.a, merge.b, merge.m);
    }
}

fn find_merges(repo: &git2::Repository, revwalk: git2::Revwalk) -> Vec<ThreeWayMerge> {
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
                o: commit.id(),
                a: parent1,
                b: parent2,
                m: base,
            }
        })
        .collect()
}

struct ThreeWayMerge {
    o: git2::Oid,
    a: git2::Oid,
    b: git2::Oid,
    m: git2::Oid,
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
// The repository type has a merge_base method which finds a base for two given commits.
