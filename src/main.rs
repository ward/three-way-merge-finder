use clap::{Arg, Command};
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let matches = cli().get_matches();

    if let Some(find_bug_fix_matches) = matches.subcommand_matches("find-bug-fix") {
        handle_find_fix(find_bug_fix_matches);
    } else if let Some(find_merge_matches) = matches.subcommand_matches("find-merges") {
        handle_find_merges(find_merge_matches);
    }
}

fn cli() -> clap::App<'static> {
    Command::new("Merge Finder")
        .version("0.4.0")
        .author("Ward Muylaert <ward.muylaert@gmail.com>")
        .about("Find 3-way merges in a git repository.")
        .subcommand(Command::new("find-merges")
                    .arg(
                        Arg::new("GITREPO")
                        .help("Give the path of an existing local git repository.")
                        .required(true),
                        )
                    .arg(
                        Arg::new("output-folder")
                        .long("output-folder")
                        .help("Specify a folder in which to place the details of merges. This information will not be produced if this parameter is not present.")
                        .takes_value(true),
                        )
                    .arg(
                        Arg::new("before")
                        .long("before")
                        .help("Specify a certain number of seconds since the UNIX epoch. Only merge commits made before this time will be used.")
                        .takes_value(true)
                        )
                    .arg(
                        Arg::new("distinct-o")
                        .long("distinct-o")
                        .help("Avoid merges where O is the same commit as A (or the same commit as B). These are trivial merges.")
                        .takes_value(false)
                        )
                    .arg(
                        Arg::new("touches-same-file")
                        .long("touches-same-file")
                        .help("Only find merges where A and B have changed the same file at least once.")
                        .takes_value(false)
                        )
                    .arg(
                        Arg::new("all-files")
                        .long("all-files")
                        .help("Copy all files present in either O, A, B, or M of the three way merge, not just those present in each and changed")
                        )
                    )
        .subcommand(Command::new("find-bug-fix")
                    .arg(
                        Arg::new("GITREPO")
                        .help("Give the path of an existing local git repository.")
                        .required(true),
                        )
                    .arg(
                        Arg::new("commitlist")
                        .long("commitlist")
                        .help("File listing merge commits, as created by this tool. For each of the merge commits, the tool will look for bug fixing commits. Results are written to a csv file. givencommit,bugfix1,bugfix2,bugfix3. Last three may be empty.")
                        .takes_value(true),
                        )
                    .arg(
                        Arg::new("commitfolder")
                        .long("commitfolder")
                        .help("A folder that is the result of finding three way merges. Each of the subfolders represents a three way merge and is named by the hash of the merge commit. This name is used to find fixing descendants. Fixing descendants are added as subfolders of a three way merge folder, alongside the existing o, a, b, m folders.")
                        .takes_value(true),
                        )
                    .arg(
                        Arg::new("fix-distance")
                        .long("fix-distance")
                        .help("Specify how 'far' away the fix can be from the merge. This is done in terms of the number of children. Currently only applies to --commitlist.")
                        .takes_value(true)
                        .default_value("10")
                        )
                    .arg(
                        Arg::new("touches-same-line")
                        .long("touches-same-line")
                        .help("Only considers bug fixing commits that also change a line that was changed between O and M. Should be terrible for recall, but hopefully ups the precision by a lot.")
                        )
                    )
}

fn handle_find_merges(matches: &clap::ArgMatches) {
    let repopath = matches.value_of("GITREPO").unwrap();
    let repo = match git2::Repository::open(repopath) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open: {}", e),
    };

    let revwalk =
        three_way_merge_finder::git_utils::create_revwalk(&repo).expect("Could not create revwalk");
    let output_folder = matches.value_of("output-folder");
    let before: Option<i64> = matches
        .value_of("before")
        .and_then(|before| before.parse().ok());
    let all_files = matches.is_present("all-files");
    let distinct_o = matches.is_present("distinct-o");
    let touches_same_file = matches.is_present("touches-same-file");

    if let Some(output_folder) = output_folder {
        three_way_merge_finder::publish::folder_dump(
            output_folder,
            &repo,
            revwalk,
            before,
            all_files,
            distinct_o,
        );
    } else {
        three_way_merge_finder::publish::print_csv_of_merges(
            &repo,
            revwalk,
            before,
            distinct_o,
            touches_same_file,
        );
    }
}

fn handle_find_fix(matches: &clap::ArgMatches) {
    let repopath = matches.value_of("GITREPO").unwrap();
    let repo = match git2::Repository::open(repopath) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open: {}", e),
    };

    if let Some(commitfolder) = matches.value_of("commitfolder") {
        three_way_merge_finder::publish::write_bug_fix_files(commitfolder, &repo);
    } else if let Some(commitfile) = matches.value_of("commitlist") {
        let fix_distance: u32 = matches.value_of("fix-distance").unwrap().parse().unwrap();

        let commitlist = read_commitlist_file(commitfile);

        if matches.is_present("touches-same-line") {
            three_way_merge_finder::publish::print_bug_fix_csv_overlapping_lines(
                &repo,
                &commitlist,
                fix_distance,
            );
        } else {
            three_way_merge_finder::publish::print_bug_fix_csv(&repo, &commitlist, fix_distance);
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
