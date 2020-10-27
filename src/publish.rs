//! Used to actually get results and print them. Makes use to the [merge](crate::merge) module.

pub fn print_csv_of_merges(repo: &git2::Repository, revwalk: git2::Revwalk, before: Option<i64>) {
    let merges = super::merge::find_merges(repo, revwalk, before);
    println!("O,A,B,M");
    for merge in merges {
        println!("{},{}", merge.to_csv_line(), merge.time(repo));
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
    for merge in merges {
        let files = merge.files_to_consider(&repo);
        let merge_path = folder.join(merge.m.to_string());
        merge.write_files_to_disk(&merge_path, files, &repo);
    }
    // TODO? Create a csv file of all merges in the folder
    // TODO? Place detailed diff "overview" in a text file there
}

/// For every given broken commit, checks for fixing descendants and prints a line of the form
///
/// ```
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
