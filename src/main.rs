use clap::{Command, Arg};
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let matches = Command::new("Merge Finder")
        .version("0.3.0")
        .author("Ward Muylaert <ward.muylaert@gmail.com>")
        .about("Find 3-way merges in a git repository.")
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
            Arg::new("all-files")
                .long("all-files")
                .help("Copy all files present in either O, A, B, or M of the three way merge, not just those present in each and changed")
        )
        .subcommand(Command::new("find-bug-fix")
            .arg(
                Arg::new("commit")
                .long("commit")
                .help("Commit for which to find bug fixing commit. Results in a csv file like for --commitlist.")
                .takes_value(true),
            )
            .arg(
                Arg::new("commitlist")
                .long("commitlist")
                .help("File listing commits, one per line. For each of the commits, the tool will look for bug fixing commits. Results are written to a csv file. givencommit,bugfix1,bugfix2,bugfix3. Last three may be empty.")
                .takes_value(true),
            )
            .arg(
                Arg::new("commitfolder")
                .long("commitfolder")
                .help("A folder that is the result of finding three way merges. Each of the subfolders represents a three way merge and is named by the hash of the merge commit. This name is used to find fixing descendants. Fixing descendants are added as subfolders of a three way merge folder, alongside the existing o, a, b, m folders.")
                .takes_value(true),
            )
        )
        .get_matches();
    let repopath = matches.value_of("GITREPO").unwrap();
    let repo = match git2::Repository::open(repopath) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open: {}", e),
    };

    if let Some(find_bug_fix_matches) = matches.subcommand_matches("find-bug-fix") {
        if let Some(commitfolder) = find_bug_fix_matches.value_of("commitfolder") {
            three_way_merge_finder::publish::write_bug_fix_files(commitfolder, &repo);
        } else if let Some(commit) = find_bug_fix_matches.value_of("commit") {
            let commitlist = vec![commit.to_owned()];
            three_way_merge_finder::publish::print_bug_fix_csv(&repo, &commitlist);
        } else if let Some(commitfile) = find_bug_fix_matches.value_of("commitlist") {
            let mut content = String::new();
            let mut f = File::open(commitfile).unwrap();
            f.read_to_string(&mut content).unwrap();
            let commitlist: Vec<_> = content
                .trim()
                .split('\n')
                .map(|line| line.trim().to_owned())
                .collect();
            three_way_merge_finder::publish::print_bug_fix_csv(&repo, &commitlist);
        } else {
            eprintln!("Nothing to do");
        }
    } else {
        let revwalk =
            three_way_merge_finder::create_revwalk(&repo).expect("Could not create revwalk");
        let output_folder = matches.value_of("output-folder");
        let before: Option<i64> = matches
            .value_of("before")
            .and_then(|before| before.parse().ok());
        let all_files = matches.is_present("all-files");
        let distinct_o = matches.is_present("distinct-o");

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
                &repo, revwalk, before, distinct_o,
            );
        }
    }
}
