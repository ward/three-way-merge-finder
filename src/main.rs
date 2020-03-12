use clap::{App, Arg};
use three_way_merge_finder::debugging;

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
    let mut revwalk = match repo.revwalk() {
        Ok(revwalk) => revwalk,
        Err(e) => panic!("Could not get revwalk to walk over commits: {}", e),
    };
    // Pushing marks a commit to start traversal from
    revwalk
        .push_head()
        .expect("Failed to set HEAD as the revwalk starting point");
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL);
    debugging::diff_walk(&repo, revwalk);
    // let merges = find_merges(&repo, revwalk);
    // for merge in merges {
    //     println!("{},{},{},{}", merge.o, merge.a, merge.b, merge.m);
    //     compare_commits(&repo, &merge);
    // }
}

fn find_merges(repo: &git2::Repository, revwalk: git2::Revwalk) -> Vec<ThreeWayMerge> {
    revwalk
        .map(|oid| {
            repo.find_commit(oid.expect("Failed to get Oid"))
                .expect("Failed to turn oid into a commit")
        })
        .filter(|commit| commit.parent_count() == 2)
        .map(|commit| {
            let parent1 = commit
                .parent_id(0)
                .expect("Failed to get id for first parent.");
            let parent2 = commit
                .parent_id(1)
                .expect("Failed to get id for second parent.");
            let base = repo
                .merge_base(parent1, parent2)
                .expect("Could not find base for the two parent commits");
            ThreeWayMerge {
                o: commit.id(),
                a: parent1,
                b: parent2,
                m: base,
            }
        })
        .collect()
}

fn compare_commits(repo: &git2::Repository, twm: &ThreeWayMerge) {
    let c1 = repo
        .find_commit(twm.o)
        .expect("Should be able to find commit O");
    let t1 = c1.tree().expect("Should be able to find tree for commit O");
    let c2 = repo
        .find_commit(twm.m)
        .expect("Should be able to find commit M");
    let t2 = c2.tree().expect("Should be able to find tree for commit M");

    let mut diffoptions = git2::DiffOptions::new();
    diffoptions.minimal(true).ignore_whitespace(true);
    let diff = repo
        .diff_tree_to_tree(Some(&t1), Some(&t2), Some(&mut diffoptions))
        .expect("Should be able to diff two commits");
    println!("Diff is: {:#?}", diff.stats());
    for delta in diff.deltas() {
        println!(
            "{:?}, {:?}, {:?}",
            delta.status(),
            delta.old_file(),
            delta.new_file()
        );
    }
}

struct ThreeWayMerge {
    o: git2::Oid,
    a: git2::Oid,
    b: git2::Oid,
    m: git2::Oid,
}
