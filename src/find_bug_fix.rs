//! This module is used to find bug-fixing commits. At the time of writing this is done by looking
//! for keywords in the Git summary. Ideally it would also take into account the lines that are
//! actually changed. Williams and Spacco (2008) propose some line tracking algorithm to this
//! effect. Original SZZ did it with cvs annotate (~ git blame)

use regex::Regex;

/// Given a git repository and a certain commit. Find the descendants and keeps those with a
/// message that could indicate a bug fix.
pub fn find_bug_fixing_commits(
    repo: &git2::Repository,
    ancestor_str: &str,
) -> Result<Vec<git2::Oid>, git2::Error> {
    let ancestor_oid = git2::Oid::from_str(ancestor_str)?;

    let mut descendants = crate::git_utils::get_descendants(repo, ancestor_oid)?;

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

    // _print_oids(&repo, &descendants);

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

fn _print_oids(repo: &git2::Repository, oids: &[git2::Oid]) {
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
            _find_responsible_commits(repo, descendant).unwrap();
        }
    }
}

/// Started out by looking at the lines that were changed by the bug_fixing_commit, cause those are
/// of interest. However, using the original SZZ might be fuzzy at best in this regard. The way I
/// understand it, they look at that line number and then git blame (technically, csv annotate) the
/// line number in the previous version of the file. It's easier to implement though, obviously,
/// but if the developer was only adding a line, then doing this is meaningless.
///
/// williams2008-defects tries to solve this through some line matching bipartite graph with
/// levenstein distances, but that is evidently a little more involved to implement.
pub fn _find_responsible_commits(
    repo: &git2::Repository,
    bug_fixing_commit: &git2::Oid,
) -> Result<std::collections::HashSet<git2::Oid>, git2::Error> {
    let bug_inducing_commits = std::collections::HashSet::new();

    let commit = repo.find_commit(*bug_fixing_commit)?;

    let file_tree = commit.tree()?;

    let mut blame_options = git2::BlameOptions::new();
    blame_options.newest_commit(*bug_fixing_commit);

    println!("Tree:");
    // TODO What does this do to directories? Autowalk or we need to handle them?
    for tree_entry in file_tree.iter() {
        println!("tree_entry: {:?}", tree_entry.name());

        // Find the lines changed in the bug_fixing_commit
        let blames = repo
            .blame_file(
                std::path::Path::new(tree_entry.name().unwrap()),
                Some(&mut blame_options),
            )
            .unwrap();

        let mut lines_changed_by_fixing_commit = Vec::new();
        for blame in blames.iter() {
            if blame.final_commit_id() == *bug_fixing_commit {
                println!("BlameHunk changed in this version");
                _print_blame(&blame);
                lines_changed_by_fixing_commit
                    .push((blame.final_start_line(), blame.lines_in_hunk()));
            }
        }
        println!("{:#?}", lines_changed_by_fixing_commit);

        // Next for lines_changed_by_fixing_commit, do git blame on the parents
    }

    Ok(bug_inducing_commits)
}

fn _print_blame(blame: &git2::BlameHunk) {
    println!(
        "Blame: {}, startlines (o, f): {} {}; len: {}; boundary? {}; orig_commit_id {}",
        blame.final_commit_id(),
        blame.orig_start_line(),
        blame.final_start_line(),
        blame.lines_in_hunk(),
        blame.is_boundary(),
        blame.orig_commit_id()
    );
}
