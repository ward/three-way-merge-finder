use std::collections::HashSet;

/// Given a git repository and a certain commit. Find the first bug fixing commit candidate.
pub fn find_bug_fixing_commit(
    repo: &git2::Repository,
    ancestor_str: &str,
) -> Result<(), git2::Error> {
    println!(
        "Looking for commit: {} in repo {}",
        ancestor_str,
        repo.path().display()
    );
    let ancestor_oid = git2::Oid::from_str(ancestor_str)?;
    let ancestor = repo.find_commit(ancestor_oid)?;
    println!("Found commit {:?}", ancestor);

    match get_descendants(repo, ancestor_oid) {
        Ok(descendants) => println!("Descendants: {:#?}", descendants),
        Err(e) => eprintln!("Failed to get descendants: {}", e),
    }

    Ok(())
}

///
/// Bit convoluted way to get all the descendants of a certain commit. Doing a reversed topological
/// revwalk with HEAD as the start. Topological ensures all children have been handled before a
/// parent is handled. Thus reversed means that when you encounter a commit, all its _parents_ have
/// already been handled. THUS if I keep a set of descendants, starting with just the commit we
/// care about, then it is a matter of checking the parents of every commit I encounter. If a
/// parent appears in my descendant list, add it to the descendant list.
fn get_descendants(
    repo: &git2::Repository,
    ancestor: git2::Oid,
) -> Result<HashSet<git2::Oid>, git2::Error> {
    let mut descendants = HashSet::new();
    descendants.insert(ancestor);

    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();
    let mut sorting = git2::Sort::TOPOLOGICAL;
    sorting.insert(git2::Sort::REVERSE);
    revwalk.set_sorting(sorting).unwrap();

    for oid in revwalk {
        let oid = oid.unwrap();
        let commit = repo.find_commit(oid).unwrap();
        let parent_in_descendants = (0..commit.parent_count())
            .map(|ctr| commit.parent_id(ctr).unwrap())
            .any(|parent| descendants.contains(&parent));
        if parent_in_descendants {
            descendants.insert(oid);
        }
    }

    descendants.remove(&ancestor);

    Ok(descendants)
}
