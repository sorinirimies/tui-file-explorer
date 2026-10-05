#!/usr/bin/env bash
# Builds the throw-away demo tree used by the `tfe` VHS tapes.
#
#   /tmp/tfe-demo        browsable tree (varied sizes, symlink, archives)
#   /tmp/tfe-demo/repo   a git repository with staged / modified / untracked files
#   /tmp/tfe-demo-home   empty $HOME so demos never touch your real settings
set -euo pipefail

ROOT=/tmp/tfe-demo
rm -rf "$ROOT" "$ROOT-home"
mkdir -p "$ROOT-home" "$ROOT"/{src/app,docs,assets,archives,repo/src}

blob() { head -c "$2" /dev/zero | tr '\0' 'x' > "$1"; }

# ── sizes ────────────────────────────────────────────────────────────────────
blob "$ROOT/assets/logo.bin" 120000
blob "$ROOT/assets/video.bin" 3400000
blob "$ROOT/build.log" 850000
printf '# Demo project\n\nA tree for the tfe demos.\n' > "$ROOT/README.md"
printf '[package]\nname = "demo"\nversion = "0.1.0"\n' > "$ROOT/Cargo.toml"
printf 'remember the milk\n' > "$ROOT/notes.txt"
printf 'fn main() {\n    println!("hello");\n}\n' > "$ROOT/src/main.rs"
printf 'pub mod app;\n' > "$ROOT/src/lib.rs"
printf '// app module\n' > "$ROOT/src/app/mod.rs"
printf '# Guide\n' > "$ROOT/docs/guide.md"
for i in 1 2 3; do printf '# Chapter %s\n' "$i" > "$ROOT/docs/chapter$i.md"; done
ln -s README.md "$ROOT/latest"

# ── archives ─────────────────────────────────────────────────────────────────
python3 - "$ROOT/archives/bundle.zip" <<'PY'
import sys, zipfile
with zipfile.ZipFile(sys.argv[1], "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("bundle/README.txt", "hello from a zip\n")
    z.writestr("bundle/data/numbers.csv", "1,2,3\n" * 200)
    z.writestr("bundle/src/lib.rs", "pub fn answer() -> u32 { 42 }\n")
PY
( cd "$ROOT/docs" && COPYFILE_DISABLE=1 tar czf "$ROOT/archives/docs.tar.gz" . )

# ── git repo ─────────────────────────────────────────────────────────────────
cd "$ROOT/repo"
git init -q
git config user.email demo@example.com
git config user.name  Demo
printf 'one\n' > src/main.rs
printf 'two\n' > src/util.rs
printf 'notes\n' > TODO.md
git add -A && git commit -qm "initial"
printf 'changed\n' >> src/main.rs        # modified
printf 'fresh\n'  > scratch.txt          # untracked
printf 'new\n'    > src/added.rs
git add src/added.rs                      # staged (added)
git rm -q TODO.md                         # deleted
