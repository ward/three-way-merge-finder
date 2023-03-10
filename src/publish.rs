//! Used to actually get results and print them.

use crate::git_utils;
use crate::merge;
use std::collections::HashSet;

pub fn print_csv_of_merges(
    repo: &git2::Repository,
    revwalk: git2::Revwalk,
    before: Option<i64>,
    distinct_o: bool,
    should_touch_same_file: bool,
) {
    let merges = merge::find_merges(repo, revwalk, before);
    println!("O,A,B,M,changed_files,timestamp");
    let java_extensions = vec![".java"];
    for merge in merges {
        if distinct_o && !merge.has_distinct_o() {
            continue;
        }
        if should_touch_same_file && !merge.a_b_change_same_file(repo, &java_extensions) {
            continue;
        }
        let file_count = merge.files_to_consider(repo).len();
        println!(
            "{},{},{}",
            merge.to_csv_line(),
            file_count,
            merge.time(repo)
        );
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
    before: Option<i64>,
    all_files: bool,
    distinct_o: bool,
) {
    let folder = folder.as_ref();
    // Create folder if needed and check it is empty
    std::fs::create_dir_all(folder).expect("Could not create output-folder");
    let mut dir_contents = std::fs::read_dir(folder).expect("Could not read output-folder");
    if dir_contents.next().is_some() {
        panic!("Specified output-folder is not empty. Aborting.");
    }

    let merges = merge::find_merges(repo, revwalk, before);

    // Create merge-hash folder and its o, a, b, and m subfolders.
    if all_files {
        for merge in merges {
            if distinct_o && !merge.has_distinct_o() {
                continue;
            }
            let merge_path = folder.join(merge.m.to_string());
            merge.write_all_files_to_disk(merge_path, repo);
        }
    } else {
        for merge in merges {
            if distinct_o && !merge.has_distinct_o() {
                continue;
            }
            let files = merge.files_to_consider(repo);
            let merge_path = folder.join(merge.m.to_string());
            merge.write_files_to_disk(&merge_path, files, repo);
        }
    }
    // TODO? Create a csv file of all merges in the folder
    // TODO? Place detailed diff "overview" in a text file there
}


// TODO Might want to move some of the following logic to find_bug_fix.

/// For every given broken commit, checks for fixing descendants and prints a line of the form
///
/// ```text
/// brokencommit,bugfix1,bugfix2,bugfix3
/// ```
///
/// The latter three may be empty.
pub fn print_bug_fix_csv(
    repo: &git2::Repository,
    broken_commit_list: &[(String, String, String, String)],
    fix_distance: u32,
) {
    for commit in broken_commit_list {
        let (o_commit, a_commit, b_commit, m_commit) = commit;
        let o_commit_oid = git2::Oid::from_str(o_commit).unwrap();
        let a_commit_oid = git2::Oid::from_str(a_commit).unwrap();
        let b_commit_oid = git2::Oid::from_str(b_commit).unwrap();
        let m_commit_oid = git2::Oid::from_str(m_commit).unwrap();
        let o_to_a = git_utils::changed_filenames(repo, &o_commit_oid, &a_commit_oid);
        let o_to_b = git_utils::changed_filenames(repo, &o_commit_oid, &b_commit_oid);
        // Keep a list of files changed in O→A AND in O→B
        let merge_changes: HashSet<_> = o_to_a
            .intersection(&o_to_b)
            // Need to get to_owned to get intersection working later...
            .map(|filename| filename.to_owned())
            .collect();
        match crate::find_bug_fix::find_bug_fixing_commits(repo, m_commit) {
            Ok(descendants) => {
                let descendants: Vec<_> = descendants
                    .iter()
                    .filter(|child| {
                        // Only keep descendants if they occur within fix_distance steps of the merge
                        git_utils::within_n_generations(repo, &m_commit_oid, child, fix_distance)
                    })
                    // Also want to filter based on overlapping files. Blames might be too selective
                    // since we are hoping for semantic stuff. Heck, overlapping files might be too
                    // selective but here we are. Downside: need the O information for that. So either
                    // recalc that here or adjust code to be able to use our OABM output instead.
                    // Leaning towards the latter since that is how we get our merges to begin with
                    // anyway.
                    // TODO: Also only keeping java files. Thought I had it but turns out no. Seems
                    // that was only in the original merge finding.
                    // TODO: So that means the total merge count also does not mean much then?
                    .filter(|child| {
                        let child_commit = repo.find_commit(**child).unwrap();
                        if child_commit.parent_count() != 1 {
                            return false;
                        }
                        let bfc_parent = child_commit.parent_id(0).unwrap();

                        // Keep bugfixing commit if changed file was also changed in O→A AND in O→B
                        let bugfix_changes = git_utils::changed_filenames(repo, &bfc_parent, child);
                        merge_changes.intersection(&bugfix_changes).next().is_some()
                    })
                    .collect();
                print_merge_bugfix_csv_line(
                    m_commit,
                    descendants.get(0),
                    descendants.get(1),
                    descendants.get(2),
                );
            }
            Err(e) => eprintln!(
                "Failed to find bug fixing commit for {}.\nError: {}",
                m_commit, e
            ),
        }
    }
}

/// Quick helper function for print_bug_fix_csv. Hence the ugly && that idk immediately how to
/// solve.
fn print_merge_bugfix_csv_line(
    m_commit: &str,
    bugfix1: Option<&&git2::Oid>,
    bugfix2: Option<&&git2::Oid>,
    bugfix3: Option<&&git2::Oid>,
) {
    println!(
        "{},{},{},{}",
        m_commit,
        bugfix1.map(|oid| oid.to_string()).unwrap_or_default(),
        bugfix2.map(|oid| oid.to_string()).unwrap_or_default(),
        bugfix3.map(|oid| oid.to_string()).unwrap_or_default(),
    );
}

/// For every given broken commit, checks for fixing descendants and prints a line of the form
///
/// ```text
/// brokencommit,bugfix1,bugfix2,bugfix3
/// ```
///
/// The latter three may be empty.
///
/// Bug fixing commit must be within fix_distance of the merge. At least one line it changes must
/// be changed in O→M.
pub fn print_bug_fix_csv_overlapping_lines(
    repo: &git2::Repository,
    broken_commit_list: &[(String, String, String, String)],
    fix_distance: u32,
) {
    for commit in broken_commit_list {
        let (o_commit, _a_commit, _b_commit, m_commit) = commit;
        let o_commit_oid = git2::Oid::from_str(o_commit).unwrap();
        // let a_commit_oid = git2::Oid::from_str(a_commit).unwrap();
        // let b_commit_oid = git2::Oid::from_str(b_commit).unwrap();
        let m_commit_oid = git2::Oid::from_str(m_commit).unwrap();
        match crate::find_bug_fix::find_bug_fixing_commits(repo, m_commit) {
            Ok(descendants) => {
                let descendants: Vec<_> = descendants
                    .iter()
                    .filter(|child| {
                        // Only keep descendants if they occur within fix_distance steps of the merge
                        git_utils::within_n_generations(repo, &m_commit_oid, child, fix_distance)
                    })
                    // Try to filter on overlapping diffs/blames
                    .filter(|child| {
                        let child_commit = repo.find_commit(**child).unwrap();
                        if child_commit.parent_count() != 1 {
                            return false;
                        }
                        let bfc_parent = child_commit.parent_id(0).unwrap();

                        git_utils::changed_same_line(
                            repo,
                            &o_commit_oid,
                            &m_commit_oid,
                            &bfc_parent,
                            child,
                        )
                    })
                    .collect();

                // Output merge commit id and its bugfixes as CSV.
                print_merge_bugfix_csv_line(
                    m_commit,
                    descendants.get(0),
                    descendants.get(1),
                    descendants.get(2),
                );
            }
            Err(e) => eprintln!(
                "Failed to find bug fixing commit for {}.\nError: {}",
                m_commit, e
            ),
        }
    }
}

/// Expects a folder that is the result of the merge commit search. Thus this folder has several
/// folders, each representing a merge commit in name. For example:
///
/// ```text
/// % tree -L 1
/// .
/// ├── 05962982b86924bb60eecbe9dc208004e39372f4
/// ├── 0777cb69301a86fa63128eca0544b970825065ad
/// ├── 2faad6b8440ea5f1494eeb046f79637286c81dc3
/// ├── 4a0b5de940b5db0855d0de052f03c98cd518f9e3
/// ├── 906f629e5dae3cd98557814b1aa424442467e8e9
/// ├── ac24fea9822dc8a4cdd49711680c608ce12f0345
/// └── d0c8a79c92c4e770a28604569a1e0860a4a0320c
/// ```
///
/// For each of the folders, uses the name as a commit to find a bug fix for. If bug fixes are
/// found, they are added as subfolders in that folder. The bug fix folder is thus a sibling to the
/// existing o, a, b, m folders. Files present in m are used as the basis of what files to write
/// out from the bug fixing commit.
///
/// If the folders already exist, the files it finds in this run will be overriden. Nothing else
/// will be touched.
pub fn write_bug_fix_files<P>(folder: P, repo: &git2::Repository)
where
    P: AsRef<std::path::Path>,
{
    let folder = folder.as_ref();
    for commit_folder in folder.read_dir().unwrap().flatten() {
        let commit_folder = commit_folder.path();
        if let Some(commit_name) = commit_folder.file_name().and_then(|osstr| osstr.to_str()) {
            match crate::find_bug_fix::find_bug_fixing_commits(repo, commit_name) {
                Ok(descendants) => {
                    let files_to_consider: HashSet<String> =
                        crate::relative_files::RelativeFiles::open(&commit_folder.join("m"))
                            .filter_map(|path| path.to_str().map(|s| s.to_owned()))
                            .collect();
                    if let Some(bug_fix_1) = descendants.get(0) {
                        git_utils::write_files_from_commit_to_disk(
                            commit_folder.join("bf1"),
                            *bug_fix_1,
                            repo,
                            &files_to_consider,
                            "BF1",
                        );
                    }
                    if let Some(bug_fix_2) = descendants.get(1) {
                        git_utils::write_files_from_commit_to_disk(
                            commit_folder.join("bf2"),
                            *bug_fix_2,
                            repo,
                            &files_to_consider,
                            "BF2",
                        );
                    }
                    if let Some(bug_fix_3) = descendants.get(2) {
                        git_utils::write_files_from_commit_to_disk(
                            commit_folder.join("bf3"),
                            *bug_fix_3,
                            repo,
                            &files_to_consider,
                            "BF3",
                        );
                    }

                    // Output a CSV to STDOUT
                    print_merge_bugfix_csv_line(
                        commit_name,
                        descendants.get(0).as_ref(),
                        descendants.get(1).as_ref(),
                        descendants.get(2).as_ref(),
                    );
                }
                Err(e) => eprintln!(
                    "Failed to find bug fixing commit for {}.\nError: {}",
                    commit_name, e
                ),
            }
        }
    }
}
