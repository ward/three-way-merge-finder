use clap::{App, Arg, SubCommand};
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let matches = App::new("Merge Finder")
        .version("0.2.0")
        .author("Ward Muylaert <ward.muylaert@gmail.com>")
        .about("Find 3-way merges in a git repository.")
        .arg(
            Arg::with_name("GITREPO")
                .help("Give the path of an existing local git repository.")
                .required(true),
        )
        .arg(
            Arg::with_name("output-folder")
                .long("output-folder")
                .help("Specify a folder in which to place the details of merges. This information will not be produced if this parameter is not present.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("before")
            .long("before")
            .help("Specify a certain number of seconds since the UNIX epoch. Only merge commits made before this time will be used.")
            .takes_value(true)
        )
        .subcommand(SubCommand::with_name("find-bug-fix")
            .arg(
                Arg::with_name("commit")
                .long("commit")
                .help("Commit for which to find bug fixing commit. Results in a csv file like for --commitlist.")
                .takes_value(true),
            )
            .arg(
                Arg::with_name("commitlist")
                .long("commitlist")
                .help("File listing commits, one per line. For each of the commits, the tool will look for bug fixing commits. Results are written to a csv file. givencommit,bugfix1,bugfix2,bugfix3. Last three may be empty.")
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
        let commitlist: Vec<_> = if let Some(commit) = find_bug_fix_matches.value_of("commit") {
            vec![commit.to_owned()]
        } else if let Some(commitfile) = find_bug_fix_matches.value_of("commitlist") {
            let mut content = String::new();
            let mut f = File::open(commitfile).unwrap();
            f.read_to_string(&mut content).unwrap();
            content
                .split('\n')
                .map(|line| line.trim().to_owned())
                .collect()
        } else {
            eprintln!("Nothing to do");
            vec![]
        };

        let mut result: Vec<String> = vec![];
        for commit in commitlist {
            match three_way_merge_finder::find_bug_fix::find_bug_fixing_commits(&repo, &commit) {
                Ok(descendants) => {
                    result.push(format!(
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
                    ));
                }
                Err(e) => eprintln!(
                    "Failed to find bug fixing commit for {}.\nError: {}",
                    commit, e
                ),
            }
        }
        let result = result.join("\n");
        // TODO Write away result
        println!("{}", result);
    } else {
        let revwalk =
            three_way_merge_finder::create_revwalk(&repo).expect("Could not create revwalk");
        let output_folder = matches.value_of("output-folder");
        let before: Option<i64> = matches
            .value_of("before")
            .and_then(|before| before.parse().ok());

        if let Some(output_folder) = output_folder {
            three_way_merge_finder::publish::folder_dump(output_folder, &repo, revwalk, before);
        } else {
            three_way_merge_finder::publish::print_csv_of_merges(&repo, revwalk, before);
        }
    }
}
