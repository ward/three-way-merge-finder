# Had quickly thrown this together in a jupyter notebook in June-July 2022.
# Just something to make the two csv files per project (a list of merges and a
# list of bugfixes) more easily accessible by creating some HTML pages listing:
# projects->merges->some info on each merge.
#
# Keeping things in a Jupyter notebook is clumsy though, so now that I need to
# run things again / have a chance for some improvement, I'm creating this
# Python file instead.
#
# Steps:
# 1. One to one copy of the notebook
# 2. Abstract away some configuration / input (eg filenames, folder names, ...)
# 3. Add some improvements that I noticed working with it
# 4. (Probably not worth the effort) Properly integrate it into this project by
#    redoing it in Rust.
################################################################################
################################################################################
################################################################################

from pathlib import Path
from tqdm import tqdm
import subprocess


# Take csv files generated by twm-finder and create a bunch of html files
# instead.

PROJECT_FOLDER = "/Users/wardmuylaert/prog/cavaldata/repos-no-checkout/repos/"
FIX_OVERVIEW_FILENAME = "fix-counts.csv"
BUGFIX_CSVS = "bugfixes/{}.csv" # {} will be project name
MERGE_CSVS = "merges/{}.csv" # {} will be project name

# Can only change the folder name of these. The others are being linked to.
# TODO: Create that folder in this script
# TODO: Make just that folder the configuration, not every file.
OUT_INDEX = "html-overview/index.html"
OUT_PROJECT = "html-overview/{}.html" # {} project name
OUT_MERGE = "html-overview/{}.{}.html" # {} project, mergehash

# First, create the index file.

# Goddamnit, fix-counts.csv was apparently created by some other notebook (in
# Descriptive Statistics).
# TODO: Incorporate that code too.
fix_overview_file = Path(FIX_OVERVIEW_FILENAME)
all_projects = []
html_rows = []
with fix_overview_file.open() as fix_overview:
    content = fix_overview.read()
    lines = content.strip().splitlines()
    for line in lines[1:]:
        project, merge_count, fix_count, rest = line.split(sep=",", maxsplit=3)
        row = "<tr><td><a href='{}.html'>{}</a></td><td>{}</td><td>{}</td></tr>".format(
            project, project, merge_count, fix_count
        )
        html_rows.append(row)
        all_projects.append(project)
index_html = Path(OUT_INDEX)
with index_html.open("w") as index:
    print("<!DOCTYPE html>\n<html>\n<body>\n", file=index)
    print(
        "<table>\n<tr><th>Project</th><th>Merge count</th><th>Immediate fix count</th></tr>",
        file=index,
    )
    print("\n".join(html_rows), file=index)
    print("</table>\n", file=index)
    print("</body></html>", file=index)

# Now, create every project's file.

# Becomes a list of:
# (project, (a_merge))
# where a_merge is
# (o, a, b, m, fix)
# Gathering these so I can see some progress (with tqdm) when generating individual merge's pages afterwards.
project_merges = []

for project in tqdm(all_projects):
    # Read in merges and their immediate fixes
    bugfixes_file = Path(BUGFIX_CSVS.format(project))
    merges_and_fixes = []
    with bugfixes_file.open() as bugfixes:
        content = bugfixes.read()
        lines = content.strip().splitlines()
        for line in lines:
            merge_commit, fix_commit, rest = line.split(sep=",", maxsplit=2)
            if fix_commit != "":
                merges_and_fixes.append((merge_commit, fix_commit))

    # Read in merges file for the O, A, B info.
    merge_info_file = Path(MERGE_CSVS.format(project))
    full_merge_info = []
    with merge_info_file.open() as merge_info:
        content = merge_info.read()
        lines = content.strip().splitlines()
        for line in lines:
            o, a, b, m, rest = line.split(",", maxsplit=4)
            for (merge, fix) in merges_and_fixes:
                if merge == m:
                    full_merge_info.append((o, a, b, m, fix))

    # Generate per project HTML
    project_html_file = Path(OUT_PROJECT.format(project))
    with project_html_file.open("w") as project_html:
        print("<!DOCTYPE><html><body>", file=project_html)
        print("<h1>{}</h1>".format(project), file=project_html)
        print(
            "<table><tr><th>O</th><th>A</th><th>B</th><th>M</th><th>Fix</th></tr>",
            file=project_html,
        )
        for (o, a, b, m, fix) in full_merge_info:
            merge_link = "{}.{}.html".format(project, m)
            print(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td><a href='{}'>{}</a></td><td>{}</td></tr>".format(
                    o, a, b, merge_link, m, fix
                ),
                file=project_html,
            )
        print("</table></body></html>", file=project_html)

    for merge_info in full_merge_info:
        project_merges.append((project, merge_info))

# And finally, the merge file for every merge that had an immediate fix. This
# will take a bit since we are running git diff --stat three times per merge...


def git_diff_stat(project, commit_old, commit_new):
    git = subprocess.run(
        ["git", "diff", "--numstat", commit_old, commit_new],
        capture_output=True,
        cwd="{}{}".format(PROJECT_FOLDER, project),
    )
    return git.stdout.decode(encoding="utf-8")


def stat_to_files(diff_stat_output):
    return map(lambda line: line.split("\t")[2], diff_stat_output.splitlines())


def overlapping_files(diff_stat_output1, diff_stat_output2, diff_stat_output3):
    """Assumes git diff --numstat was called and takes the output as a string."""
    files1 = frozenset(stat_to_files(diff_stat_output1))
    files2 = frozenset(stat_to_files(diff_stat_output2))
    files3 = frozenset(stat_to_files(diff_stat_output3))
    return files1.intersection(files2).intersection(files3)


def write_merge_html_file(project_merge):
    (project, (o, a, b, m, fix)) = project_merge
    merge_html_file = Path(OUT_MERGE.format(project, m))
    with merge_html_file.open("w") as merge_html:
        print("<!DOCTYPE><html><body>", file=merge_html)
        print("<h2>Meta</h2>", file=merge_html)
        print("<p>{}</p>".format(project), file=merge_html)
        print("<pre>cd ../{}</pre>".format(project), file=merge_html)
        print("<pre>{},{},{},</pre>".format(project, m, fix), file=merge_html)
        print("<h2>O to A</h2>", file=merge_html)
        print("<pre>git diff --stat {} {}</pre>".format(o, a), file=merge_html)
        print("<pre>git diff {} {}</pre>".format(o, a), file=merge_html)
        print("<h2>O to B</h2>", file=merge_html)
        print("<pre>git diff --stat {} {}</pre>".format(o, b), file=merge_html)
        print("<pre>git diff {} {}</pre>".format(o, b), file=merge_html)
        print("<h2>M to Fix</h2>", file=merge_html)
        print("<pre>git diff --stat {} {}</pre>".format(m, fix), file=merge_html)
        print("<pre>git diff {} {}</pre>".format(m, fix), file=merge_html)
        print(
            "<pre>git -c delta.side-by-side=true diff {} {}</pre>".format(m, fix),
            file=merge_html,
        )

        # Running git makes it slow

        print("<h1>O to A</h1>", file=merge_html)
        git_oa = git_diff_stat(project, o, a)
        print("<pre>", file=merge_html)
        print(git_oa, file=merge_html)
        print("</pre>", file=merge_html)

        print("<h1>O to B</h1>", file=merge_html)
        git_ob = git_diff_stat(project, o, b)
        print("<pre>", file=merge_html)
        print(git_ob, file=merge_html)
        print("</pre>", file=merge_html)

        print("<h1>M to Fix</h1>", file=merge_html)
        commit_msg = subprocess.run(
            ["git", "show", "--no-patch", '--format="%B"', fix],
            capture_output=True,
            cwd="{}{}".format(PROJECT_FOLDER, project),
        )
        commit_msg = commit_msg.stdout.decode(encoding="utf-8")
        print("<pre>", file=merge_html)
        print(commit_msg, file=merge_html)
        print("</pre>", file=merge_html)
        git_mf = git_diff_stat(project, m, fix)
        print("<pre>", file=merge_html)
        print(git_mf, file=merge_html)
        print("</pre>", file=merge_html)

        print("<h1>Overlapping files</h1>", file=merge_html)
        overlap = overlapping_files(git_oa, git_ob, git_mf)
        if len(overlap) == 0:
            print("<p>No overlap.</p>", file=merge_html)
        else:
            print("<pre>", file=merge_html)
            print("\n".join(overlap), file=merge_html)
            print("</pre>", file=merge_html)

        print("</body></html>", file=merge_html)


# Running this on all 1207 merges took 1h40 the first time (ie, with three git
# diff calls). In the end I think the filter=blob:none clone was to blame.
# Every diff resulted in needing to bother the remote server.
for project_merge in tqdm(project_merges):
    write_merge_html_file(project_merge)
