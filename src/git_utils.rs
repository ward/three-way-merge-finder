use git2::{Blame, BlameOptions, Diff, DiffLineType, DiffOptions, Oid, Repository, Revwalk};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Creates a toplogical revwalk over a repository, starting at HEAD.
pub fn create_revwalk(repo: &Repository) -> Result<Revwalk, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    // Pushing marks a commit to start traversal from
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
    Ok(revwalk)
}

/// Given two Oids, finds the commits, their trees, diffs the trees.
fn diff_commits<'a>(
    repo: &'a Repository,
    old: &'a Oid,
    new: &'a Oid,
) -> Result<Diff<'a>, git2::Error> {
    let mut diffoptions = DiffOptions::new();
    diffoptions.minimal(true).ignore_whitespace(true);
    let old = repo.find_commit(*old)?;
    let old_tree = old.tree()?;
    let new = repo.find_commit(*new)?;
    let new_tree = new.tree()?;
    repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), Some(&mut diffoptions))
}

/// Given two commits (well, Oids), does a diff and returns the changed files.
pub fn changed_filenames(repo: &Repository, old: &Oid, new: &Oid) -> HashSet<String> {
    let diff = diff_commits(repo, old, new).expect("Should be able to diff old to new");
    let mut paths = HashSet::new();
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

/// Given two Oids (that have to be commits), does a diff and returns the old paths.
fn _old_paths(repo: &Repository, old: &Oid, new: &Oid) -> HashSet<PathBuf> {
    let diff = diff_commits(repo, old, new).expect("Should be able to diff old to new");
    let mut paths = HashSet::new();
    for delta in diff.deltas() {
        if let Some(path) = delta.old_file().path() {
            paths.insert(path.to_owned());
        }
    }
    paths
}

/// Given a path and two oids, looks for blames between the first and the second oid (inclusive).
fn blame_between<'a>(
    repo: &'a Repository,
    old: &Oid,
    new: &Oid,
    path: &Path,
) -> Result<Blame<'a>, git2::Error> {
    let mut opts = BlameOptions::new();
    opts.track_copies_same_file(true)
        .oldest_commit(*old)
        .newest_commit(*new);
    let blames = repo.blame_file(path, Some(&mut opts))?;

    Ok(blames)
}

/// Attempt like this to have more precision when finding bugfixes for merge commits. Ensure that
/// the bug fixing commit changes a line that was also changed in O->A, O->B, O->M. To keep things
/// simple for now, maybe just check with O->M
pub fn changed_same_line(
    repo: &Repository,
    blame_oldest: &Oid,
    blame_newest: &Oid,
    commit_old: &Oid,
    commit_new: &Oid,
) -> bool {
    let diff =
        diff_commits(repo, commit_old, commit_new).expect("Should be able to diff old to new");
    let mut changed_same_line = false;
    // Don't cache for now, some moving out of closure issues.
    // path->blame
    // let mut blames = std::collections::HashMap::new();
    println!(
        "Foreach in O {}, M {}, bugfix {}",
        blame_oldest, blame_newest, commit_new
    );
    diff.foreach(
        &mut |_, _| true,
        None,
        None,
        Some(&mut |diff_delta, _some_diff_hunk, diff_line| {
            // If we already found an overlap, don't go through all the work. Cannot return false
            // to end the iteration, because that makes the result of the foreach an error.
            if changed_same_line {
                return true;
            }

            // Much like in a git diff, there can be context and other stuff we are not interested
            // in. Abort this foreach check early enough if that is the case.
            match diff_line.origin_value() {
                DiffLineType::Context
                | DiffLineType::Binary
                | DiffLineType::AddEOFNL
                | DiffLineType::DeleteEOFNL
                | DiffLineType::ContextEOFNL => return true,
                _ => {}
            };

            // TODO: Should I consider the addition of a line _between_ changed lines?

            if let Some(path) = diff_delta.old_file().path() {
                if let Ok(path_blames) = blame_between(repo, blame_oldest, blame_newest, path) {
                    if let Some(old_lineno) = diff_line.old_lineno() {
                        // I assume that if it was changed before, then it will return a hunk,
                        // otherwise not.
                        if let Some(blame_hunk) = path_blames.get_line(old_lineno as usize) {
                            // Boundary seems to mean the blame_oldest commit was reached (in our
                            // use-case: commit O). In other words: if the boundary was reached, we
                            // do not care.
                            let is_boundary = blame_hunk.is_boundary();

                            if !is_boundary {
                                println!("{:?} {} {}", path, old_lineno, is_boundary);
                                changed_same_line = true;
                                return true;
                            }
                        }
                    }
                }
            }
            true
        }),
    )
    .expect("diff.foreach went oopsy");

    changed_same_line
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
    only_extensions: &[&str],
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
