use git2::{Oid, Repository, Revwalk};

/// Creates a toplogical revwalk over a repository, starting at HEAD.
pub fn create_revwalk(repo: &Repository) -> Result<Revwalk, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    // Pushing marks a commit to start traversal from
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
    Ok(revwalk)
}

/// Given two Oids, finds the commits, their trees, diffs the trees. Panic if Oid is not a commit
/// or if no tree is found.
fn diff_commits<'a>(
    repo: &'a Repository,
    old: &'a Oid,
    new: &'a Oid,
) -> Result<git2::Diff<'a>, git2::Error> {
    let mut diffoptions = git2::DiffOptions::new();
    diffoptions.minimal(true).ignore_whitespace(true);
    let old = repo.find_commit(*old)?;
    let old_tree = old.tree()?;
    let new = repo.find_commit(*new)?;
    let new_tree = new.tree()?;
    repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), Some(&mut diffoptions))
}

/// Given two commits (well, Oids), does a diff and returns the changed files.
pub fn changed_filenames(
    repo: &Repository,
    old: &Oid,
    new: &Oid,
) -> std::collections::HashSet<String> {
    let diff = diff_commits(repo, old, new).expect("Should be able to diff old to new");
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

/// Checks whether Δ1 and Δ2 have at least one file they both changed. You may provide a list of
/// extensions to only consider files ending in those. Empty list of extensions means all files are
/// considered.
pub fn changed_same_file(
    repo: &Repository,
    commit1_old: &Oid,
    commit1_new: &Oid,
    commit2_old: &Oid,
    commit2_new: &Oid,
    only_extensions: &Vec<&str>,
) -> bool {
    let commit1_files: std::collections::HashSet<_> =
        changed_filenames(repo, commit1_old, commit1_new)
            .into_iter()
            .filter(|filename| only_extensions.iter().any(|ext| filename.ends_with(ext)))
            .collect();
    let commit2_files: std::collections::HashSet<_> =
        changed_filenames(repo, commit2_old, commit2_new)
            .into_iter()
            .filter(|filename| only_extensions.iter().any(|ext| filename.ends_with(ext)))
            .collect();
    !commit1_files.is_disjoint(&commit2_files)
}

/// Since we are not keeping track of the parent relation when getting descendants, we need to
/// essentially redo that check. Given a commit, take the parents up to n time and see if any
/// equals the given root. An `n` of 1 here means the direct child.
pub fn within_n_generations(repo: &Repository, root: &Oid, child: &Oid, n: u32) -> bool {
    let child = repo.find_commit(*child).unwrap();
    let mut children = vec![child];
    for _ in 0..n {
        let mut ancestors = vec![];
        for child in children {
            for ancestor in child.parents() {
                if root == &ancestor.id() {
                    return true;
                }
                ancestors.push(ancestor);
            }
        }
        children = ancestors;
    }
    false
}

/// Bit convoluted way to get all the descendants of a certain commit. Doing a reversed topological
/// revwalk with HEAD as the start. Topological ensures all children have been handled before a
/// parent is handled. Thus reversed means that when you encounter a commit, all its _parents_ have
/// already been handled. THUS if I keep a set of descendants, starting with just the commit we
/// care about, then it is a matter of checking the parents of every commit I encounter. If a
/// parent appears in my descendant list, add it to the descendant list.
///
/// Not that this does imply the descendants are _not_ sorted by time, but also by topology. Within
/// one branch, this makes no difference. Across branches there is no time assumption you can make.
pub fn get_descendants(repo: &Repository, ancestor: Oid) -> Result<Vec<Oid>, git2::Error> {
    // We use Oid instead of Commit types. Commit types do not have PartialEq so would not be able
    // to use contains() further down.
    let mut descendants: Vec<Oid> = Vec::new();
    descendants.push(ancestor);

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    let mut sorting = git2::Sort::TOPOLOGICAL;
    sorting.insert(git2::Sort::REVERSE);
    revwalk.set_sorting(sorting)?;

    for oid in revwalk {
        let oid = oid.unwrap();
        let commit = repo.find_commit(oid).unwrap();
        // Clumsy way to get the parents. Commit has a parents() method, but that returns Commit
        // types while we are collecting Oids.
        let parent_in_descendants = (0..commit.parent_count())
            .map(|ctr| commit.parent_id(ctr).unwrap())
            .any(|parent| descendants.contains(&parent));
        if parent_in_descendants {
            descendants.push(oid);
        }
    }

    // The first commit we put in the vector was the ancestor. Remove it now.
    descendants.remove(0);

    Ok(descendants)
}
