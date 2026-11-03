#!/usr/bin/env nu
# Builds the throw-away demo tree used by the `tfe` VHS tapes.
#
#   <root>        browsable tree (varied sizes, symlink, archives)
#   <root>/repo   a git repository with staged / modified / untracked files
#   <root>-home   empty $HOME so demos never touch your real settings
#
# Usage: nu examples/vhs/fixture.nu [--root /tmp/tfe-demo]
# Needs git, tar and zip.

# Write a file of `size` bytes ("xxx…").
export def blob [file: path, size: int] {
    "" | fill --character "x" --width $size | save --force --raw $file
}

# The browsable tree: varied file sizes, sources, docs and a symlink.
export def tree [root: path] {
    for d in [src/app docs assets archives repo/src] { mkdir ($root | path join $d) }
    blob ($root | path join assets/logo.bin) 120000
    blob ($root | path join assets/video.bin) 3400000
    blob ($root | path join build.log) 850000
    "# Demo project\n\nA tree for the tfe demos.\n" | save --force ($root | path join README.md)
    "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n" | save --force ($root | path join Cargo.toml)
    "remember the milk\n" | save --force ($root | path join notes.txt)
    "fn main() {\n    println!(\"hello\");\n}\n" | save --force ($root | path join src/main.rs)
    "pub mod app;\n" | save --force ($root | path join src/lib.rs)
    "// app module\n" | save --force ($root | path join src/app/mod.rs)
    "# Guide\n" | save --force ($root | path join docs/guide.md)
    for i in 1..3 { $"# Chapter ($i)\n" | save --force ($root | path join $"docs/chapter($i).md") }
    ^ln -s README.md ($root | path join latest)
}

# archives/bundle.zip (a small project) and archives/docs.tar.gz (the docs folder).
export def archives [root: path] {
    let out = $root | path join archives
    let staging = mktemp --directory
    let bundle = $staging | path join bundle
    mkdir ($bundle | path join data) ($bundle | path join src)
    "hello from a zip\n" | save ($bundle | path join README.txt)
    1..200 | each { "1,2,3\n" } | str join | save ($bundle | path join data/numbers.csv)
    "pub fn answer() -> u32 { 42 }\n" | save ($bundle | path join src/lib.rs)
    do { cd $staging; ^zip -qrX ($out | path join bundle.zip) bundle }
    rm --recursive $staging
    do { cd ($root | path join docs); with-env {COPYFILE_DISABLE: "1"} { ^tar czf ($out | path join docs.tar.gz) . } }
}

# A git repository with one modified, one untracked, one staged and one deleted file.
export def repo [dir: path] {
    cd $dir
    ^git init -q
    ^git config user.email demo@example.com
    ^git config user.name Demo
    ^git config commit.gpgsign false
    "one\n" | save --force src/main.rs
    "two\n" | save --force src/util.rs
    "notes\n" | save --force TODO.md
    ^git add -A
    ^git commit -qm "initial"
    "changed\n" | save --append src/main.rs
    "fresh\n" | save --force scratch.txt
    "new\n" | save --force src/added.rs
    ^git add src/added.rs
    ^git rm -q TODO.md
}

# Rebuild everything from scratch under `root`.
export def build [root: path] {
    let home = $"($root)-home"
    rm --recursive --force $root $home
    mkdir $home
    tree $root
    archives $root
    repo ($root | path join repo)
}

def main [--root: path = "/tmp/tfe-demo"] {
    build $root
    print -e $"demo tree ready: ($root)"
}
