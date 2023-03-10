use clap::Parser;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    match Cli::parse() {
        Cli::FindMerge(find_merge) => handle_find_merges(find_merge),
        Cli::FindBugFix(find_bug_fix) => handle_find_fix(find_bug_fix),
    };
}

#[derive(Parser)]
#[command(version, author, about)]
enum Cli {
    FindMerge(FindMerge),
    FindBugFix(FindBugFix),
}

#[derive(Parser)]
struct FindMerge {
    /// Give the path of an existing local Git repository
    gitrepo: String,
    /// Specify a folder in which to place the details of merges. This information will not be
    /// produced if this parameter is not present.
    #[arg(long)]
    output_folder: Option<String>,
    /// Specify a certain number of seconds since the UNIX epoch. Only merge commits made before
    /// this time will be used.
    #[arg(long)]
    before: Option<i64>,
    /// Avoid merges where O is the same commit as A (or the same commit as B). These are trivial
    /// merges.
    #[arg(long)]
    distinct_o: bool,
    /// Only find merges where A and B have changed the same file at least once.
    #[arg(long)]
    touches_same_file: bool,
    /// Copy all files present in either O, A, B, or M of the three way merge, not just those
    /// present in each and changed
    #[arg(long)]
    all_files: bool,
}

#[derive(Parser)]
struct FindBugFix {
    /// Give the path of an existing local Git repository.
    gitrepo: String,
    /// File listing merge commits, as created by this tool. For each of the merge commits, the
    /// tool will look for bug fixing commits. Results are written to a csv file.
    /// givencommit,bugfix1,bugfix2,bugfix3. Last three may be empty.
    #[arg(long)]
    commitlist: Option<String>,
    /// A folder that is the result of finding three way merges. Each of the subfolders represents
    /// a three way merge and is named by the hash of the merge commit. This name is used to find
    /// fixing descendants. Fixing descendants are added as subfolders of a three way merge folder,
    /// alongside the existing o, a, b, m folders.
    #[arg(long)]
    commitfolder: Option<String>,
    /// Specify how 'far' away the fix can be from the merge. This is done in terms of the number
    /// of children. Currently only applies to --commitlist.
    #[arg(long, default_value_t = 10)]
    fix_distance: u32,
    /// Only considers bug fixing commits that also change a line that was changed between O and M.
    /// Should be terrible for recall, but hopefully ups the precision significantly.
    #[arg(long)]
    touches_same_line: bool,
}

fn handle_find_merges(cli: FindMerge) {
    let repo = match git2::Repository::open(cli.gitrepo) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open: {}", e),
    };
    let revwalk =
        three_way_merge_finder::git_utils::create_revwalk(&repo).expect("Could not create revwalk");

    if let Some(output_folder) = cli.output_folder {
        three_way_merge_finder::publish::folder_dump(
            output_folder,
            &repo,
            revwalk,
            cli.before,
            cli.all_files,
            cli.distinct_o,
        );
    } else {
        three_way_merge_finder::publish::print_csv_of_merges(
            &repo,
            revwalk,
            cli.before,
            cli.distinct_o,
            cli.touches_same_file,
        );
    }
}

fn handle_find_fix(cli: FindBugFix) {
    let repo = match git2::Repository::open(cli.gitrepo) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open: {}", e),
    };

    if let Some(commitfolder) = cli.commitfolder {
        three_way_merge_finder::publish::write_bug_fix_files(commitfolder, &repo);
    } else if let Some(commitfile) = cli.commitlist {
        let commitlist = read_commitlist_file(&commitfile);

        if cli.touches_same_line {
            three_way_merge_finder::publish::print_bug_fix_csv_overlapping_lines(
                &repo,
                &commitlist,
                cli.fix_distance,
            );
        } else {
            three_way_merge_finder::publish::print_bug_fix_csv(
                &repo,
                &commitlist,
                cli.fix_distance,
            );
        }
    } else {
        eprintln!("Nothing to do");
    }
}

/// Reads in a CSV file of the form O,A,B,M SHAs.
fn read_commitlist_file(filename: &str) -> Vec<(String, String, String, String)> {
    // Read in the commitlist file
    let mut content = String::new();
    let mut f = File::open(filename).unwrap();
    f.read_to_string(&mut content).unwrap();
    content
        .trim()
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut split_line = line.trim().split(',');
            (
                split_line.next().expect("Should be an O commit").to_owned(),
                split_line.next().expect("Should be an A commit").to_owned(),
                split_line.next().expect("Should be an B commit").to_owned(),
                split_line.next().expect("Should be an M commit").to_owned(),
            )
        })
        .collect()
}
