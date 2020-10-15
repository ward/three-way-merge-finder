use clap::{App, Arg, SubCommand};

fn main() {
    let matches = App::new("Merge Finder")
        .version("0.1.0")
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
            Arg::with_name("COMMIT")
            .help("Commit for which to find bug fixing commit")
            .required(true),
        )
        )
        .get_matches();
    let repopath = matches.value_of("GITREPO").unwrap();
    let repo = match git2::Repository::open(repopath) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open: {}", e),
    };
    let revwalk = three_way_merge_finder::create_revwalk(&repo).expect("Could not create revwalk");
    if let Some(find_bug_fix_matches) = matches.subcommand_matches("find-bug-fix") {
        let commit = find_bug_fix_matches.value_of("COMMIT").unwrap();
        match three_way_merge_finder::find_bug_fix::find_bug_fixing_commit(&repo, commit) {
            Ok(()) => {}
            Err(e) => eprintln!("Failed to find bug fixing commit.\nError: {}", e),
        }
    } else {
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
