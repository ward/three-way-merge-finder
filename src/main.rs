use clap::{App, Arg};
use three_way_merge_finder::debugging;
use three_way_merge_finder::merge::*;

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
        .get_matches();
    let repopath = matches.value_of("GITREPO").unwrap();
    let repo = match git2::Repository::open(repopath) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open: {}", e),
    };
    let revwalk = three_way_merge_finder::create_revwalk(&repo).expect("Could not create revwalk");
    // debugging::diff_walk(&repo, revwalk);
    let merges = find_merges(&repo, revwalk);
    for merge in merges {
        println!("{},{},{},{}", merge.o, merge.a, merge.b, merge.m);
        compare_commits(&repo, &merge);
    }
}
