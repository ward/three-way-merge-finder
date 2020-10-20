//! This module is used to find bug-fixing commits. At the time of writing this is done by looking
//! for keywords in the Git summary. Ideally it would also take into account the lines that are
//! actually changed. Williams and Spacco (2008) propose some line tracking algorithm to this
//! effect.

use regex::Regex;

/// Given a git repository and a certain commit. Find the first bug fixing commit candidate.
pub fn find_bug_fixing_commits(
    repo: &git2::Repository,
    ancestor_str: &str,
) -> Result<Vec<git2::Oid>, git2::Error> {
    println!(
        "Looking for commit: {} in repo {}",
        ancestor_str,
        repo.path().display()
    );
    let ancestor_oid = git2::Oid::from_str(ancestor_str)?;
    let ancestor = repo.find_commit(ancestor_oid)?;
    println!("Found commit {:?}", ancestor);

    let mut descendants = get_descendants(repo, ancestor_oid)?;

    // Need to collect into a Vec, otherwise all the iterators retain the immutable borrow on
    // descendants for too long.
    let not_a_fix: Vec<_> = descendants
        .iter()
        .enumerate()
        .filter(|(_ctr, descendant)| match repo.find_commit(**descendant) {
            Ok(commit) => {
                let summary = commit.summary().unwrap_or("");
                !potential_bug_fix_summary(summary)
            }
            Err(e) => {
                eprintln!(
                    "Failed to find commit for descendant {} ??? This should not happen. Error: {}",
                    descendant, e
                );
                true
            }
        })
        .map(|(ctr, _)| ctr)
        // Rev is important! If we remove from the front, then the indices that come after are no
        // longer valid
        .rev()
        .collect();

    for idx in not_a_fix {
        descendants.remove(idx);
    }

    _print_oids(&repo, &descendants);

    Ok(descendants)
}

/// Doing this by means of the text in the summary. There are some methods available. Leaning
/// towards Ray et al 2016 since it is easier.
///
/// # SZZ (Sliwerski et al 2005)
///
/// Split up in a stream of tokens and match it to the bug reports they got from elsewhere.
///
/// > * a bug number, if it matches one of the following regular ex- pressions (given in FLEX syntax):
/// >     * bug[# \t]*[0-9]+,
/// >     * pr[# \t]*[0-9]+,
/// >     * show\_bug\.cgi\?id=[0-9]+, or
/// >     * \[[0-9]+\]
/// > * a plain number, if it is a string of digits [0-9]+
/// > * a keyword, if it matches the following regular expression:
/// >       fix(e[ds])?|bugs?|defects?|patch
/// > * a word, if it is a string of alphanumeric characters
///
/// # Ray et al 2016
///
/// > Then similar to Mockus et al. [33], we marked a commit as a bug-fix, if the corresponding
/// > stemmed bag-of-words contains at least one of the error related keywords: ‘error’, ‘bug’,
/// > ‘fix’, ‘issue’, ‘mistake’, ‘incorrect’, ‘fault’, ‘defect’, ‘flaw’, and ‘type’.
///
/// # Karampatsis et al 2020
///
/// > To decide if a commit fixes a bug, we checked if its commit message contains at least one of the
/// > keywords: ‘error’, ‘bug’, ‘fix’, ‘issue’, ‘mistake’, ‘incorrect’, ‘fault’, ‘defect’, ‘flaw’, and
/// > ‘type’. This heuristic was previously used by Ray et al. [21] and was shown to achieve 96%
/// > accuracy on a set of 300 manually verified commits and 97.6% on a set of 384 manually verified
/// > commits [25]
///
/// # Mockus et al 2000
///
/// > We envisioned three primary types of maintenance: fault fixes for keywords such as,
/// > problem, incorrect, correct; new code development for keywords add, new, mod, update; and
/// > code improvement for keywords cleanup, unneeded, remove, rework.
fn potential_bug_fix_summary(summary: &str) -> bool {
    lazy_static! {
        static ref SZZ_MATCHERS: Vec<Regex> = vec![
            Regex::new("(?i)fix(?:e[ds])?").unwrap(),
            Regex::new("(?i)bugs?").unwrap(),
            Regex::new("(?i)defects?").unwrap(),
            Regex::new("(?i)patch").unwrap(),
        ];
        static ref RAY_MATCHERS: Vec<Regex> = vec![
            Regex::new("(?i)error").unwrap(),
            Regex::new("(?i)bug").unwrap(),
            Regex::new("(?i)fix").unwrap(),
            Regex::new("(?i)issue").unwrap(),
            Regex::new("(?i)mistake").unwrap(),
            Regex::new("(?i)incorrect").unwrap(),
            Regex::new("(?i)fault").unwrap(),
            Regex::new("(?i)defect").unwrap(),
            Regex::new("(?i)flaw").unwrap(),
            Regex::new("(?i)type").unwrap(),
        ];
    }
    RAY_MATCHERS.iter().any(|matcher| matcher.is_match(summary))
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
fn get_descendants(
    repo: &git2::Repository,
    ancestor: git2::Oid,
) -> Result<Vec<git2::Oid>, git2::Error> {
    let mut descendants = Vec::new();
    descendants.push(ancestor);

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
            descendants.push(oid);
        }
    }

    // The first commit we put in the vector was the ancestor. Remove it now.
    descendants.remove(0);

    Ok(descendants)
}

fn _print_oids(repo: &git2::Repository, oids: &Vec<git2::Oid>) {
    for descendant in oids {
        if let Ok(commit) = repo.find_commit(*descendant) {
            let summary = commit.summary().unwrap_or("");
            let is_bug_fix = potential_bug_fix_summary(summary);
            let time = commit.time().seconds();
            println!(
                "[{}] {}: {} (fix? {})",
                time,
                commit.id(),
                summary,
                is_bug_fix
            );
        }
    }
}
