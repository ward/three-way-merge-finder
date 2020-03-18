use clap::{App, Arg};

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
        .get_matches();
    let repopath = matches.value_of("GITREPO").unwrap();
    let output_folder = matches.value_of("output-folder");
    let repo = match git2::Repository::open(repopath) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open: {}", e),
    };
    let revwalk = three_way_merge_finder::create_revwalk(&repo).expect("Could not create revwalk");

    if let Some(output_folder) = output_folder {
        three_way_merge_finder::publish::folder_dump(output_folder, &repo, revwalk);
    } else {
        three_way_merge_finder::publish::print_csv_of_merges(&repo, revwalk);
    }
}
