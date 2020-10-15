use std::collections::HashSet;

/// Given a git repository and a certain commit. Find the first bug fixing commit candidate.
pub fn find_bug_fixing_commit(
    repo: &git2::Repository,
    ancestor_str: &str,
) -> Result<(), git2::Error> {
    // TODO Not 100% sure I will need that generic revwalk.
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

/// Bit convoluted way to get all the descendants of a certain commit. Cannot just use revwalk
/// since that only goes up the parent chain when starting at a certain commit.
///
/// Possible improvement: Do topological revwalk from HEAD, but reverse it. Topological ensures
/// that all children have been handled before a parent is handled. Thus reversed means that when
/// you encounter a commit, all its _parents_ have already been handled. THUS if I keep a set of
/// descendants, starting with just the commit we care about, then it is a matter of checking the
/// parents of every commit I encounter. If a parent appears in my descendant list, add it to the
/// descendant list. In this manner I can still Revwalk. Question: Is this better? Supposedly the
/// revwalk first does all the parent stuff I do manually here, so that would not help much...
fn get_descendants(
    repo: &git2::Repository,
    ancestor: git2::Oid,
) -> Result<HashSet<git2::Oid>, git2::Error> {
    let head = repo.head()?.target().unwrap();

    let mut descendants = HashSet::new();

    // Loop through worklist, find parents for each head, add head to the children, add the combo
    // to the worklist.
    let mut worklist = vec![(head, vec![])];
    while !worklist.is_empty() {
        let (next, mut children) = worklist.pop().unwrap();
        if next == ancestor {
            descendants.extend(children);
            continue;
        }
        let next_c = repo.find_commit(next).unwrap();
        children.push(next);
        (0..next_c.parent_count())
            .map(|ctr| next_c.parent_id(ctr).unwrap())
            .for_each(|parent| {
                worklist.push((parent, children.clone()));
            });
    }
    Ok(descendants)
}
