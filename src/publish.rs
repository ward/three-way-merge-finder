//! Used to actually get results and print them. Makes use of the [merge](crate::merge) module.

pub fn print_csv_of_merges(
    repo: &git2::Repository,
    revwalk: git2::Revwalk,
    before: Option<i64>,
    distinct_o: bool,
) {
    let merges = super::merge::find_merges(repo, revwalk, before);
    println!("O,A,B,M,changed_files,timestamp");
    for merge in merges {
        if distinct_o && !merge.has_distinct_o() {
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
    std::fs::create_dir_all(&folder).expect("Could not create output-folder");
    let mut dir_contents = std::fs::read_dir(&folder).expect("Could not read output-folder");
    if dir_contents.next().is_some() {
        panic!("Specified output-folder is not empty. Aborting.");
    }

    let merges = super::merge::find_merges(repo, revwalk, before);

    // Create merge-hash folder and its o, a, b, and m subfolders.
    if all_files {
        for merge in merges {
            if distinct_o && !merge.has_distinct_o() {
                continue;
            }
            let merge_path = folder.join(merge.m.to_string());
            merge.write_all_files_to_disk(merge_path, &repo);
        }
    } else {
        for merge in merges {
            if distinct_o && !merge.has_distinct_o() {
                continue;
            }
            let files = merge.files_to_consider(&repo);
            let merge_path = folder.join(merge.m.to_string());
            merge.write_files_to_disk(&merge_path, files, &repo);
        }
    }
    // TODO? Create a csv file of all merges in the folder
    // TODO? Place detailed diff "overview" in a text file there
}

/// For every given broken commit, checks for fixing descendants and prints a line of the form
///
/// ```text
/// brokencommit,bugfix1,bugfix2,bugfix3
/// ```
///
/// The latter three may not be present.
pub fn print_bug_fix_csv(repo: &git2::Repository, broken_commit_list: &[String]) {
    for commit in broken_commit_list {
        match crate::find_bug_fix::find_bug_fixing_commits(&repo, &commit) {
            Ok(descendants) => {
                println!(
                    "{},{},{},{}",
                    commit,
                    descendants
                        .get(0)
                        .map(|oid| oid.to_string())
                        .unwrap_or_default(),
                    descendants
                        .get(1)
                        .map(|oid| oid.to_string())
                        .unwrap_or_default(),
                    descendants
                        .get(2)
                        .map(|oid| oid.to_string())
                        .unwrap_or_default(),
                );
            }
            Err(e) => eprintln!(
                "Failed to find bug fixing commit for {}.\nError: {}",
                commit, e
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
    for commit_folder in folder.read_dir().unwrap() {
        if let Ok(commit_folder) = commit_folder {
            let commit_folder = commit_folder.path();
            if let Some(commit_name) = commit_folder.file_name().and_then(|osstr| osstr.to_str()) {
                match crate::find_bug_fix::find_bug_fixing_commits(&repo, &commit_name) {
                    Ok(descendants) => {
                        let files_to_consider: std::collections::HashSet<String> =
                            crate::relative_files::RelativeFiles::open(&commit_folder.join("m"))
                                .map(|path| path.to_str().map(|s| s.to_owned()))
                                .flatten()
                                .collect();
                        if let Some(bug_fix_1) = descendants.get(0) {
                            crate::merge::write_files_from_commit_to_disk(
                                commit_folder.join("bf1"),
                                *bug_fix_1,
                                repo,
                                &files_to_consider,
                                "BF1",
                            );
                        }
                        if let Some(bug_fix_2) = descendants.get(1) {
                            crate::merge::write_files_from_commit_to_disk(
                                commit_folder.join("bf2"),
                                *bug_fix_2,
                                repo,
                                &files_to_consider,
                                "BF2",
                            );
                        }
                        if let Some(bug_fix_3) = descendants.get(2) {
                            crate::merge::write_files_from_commit_to_disk(
                                commit_folder.join("bf3"),
                                *bug_fix_3,
                                repo,
                                &files_to_consider,
                                "BF3",
                            );
                        }

                        // Output a CSV to STDOUT
                        println!(
                            "{},{},{},{}",
                            commit_name,
                            descendants
                                .get(0)
                                .map(|oid| oid.to_string())
                                .unwrap_or_default(),
                            descendants
                                .get(1)
                                .map(|oid| oid.to_string())
                                .unwrap_or_default(),
                            descendants
                                .get(2)
                                .map(|oid| oid.to_string())
                                .unwrap_or_default(),
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
}
