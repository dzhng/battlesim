#!/bin/sh
# Vercel's install step (vercel.json). The project's Git LFS setting is off, so
# the clone carries pointer files; fetch only the objects the site serves (the
# runtime assets and the effect flipbooks the bundle imports) rather than every
# source, reference and evidence file. vercel-build.sh fails the build if the
# site it built still carries a pointer, so a new LFS import shows up there.
set -eu

# Vercel's clone has no remote and no LFS filter: install the filter so the
# pull writes files, and name the (public) repository's LFS store.
git lfs install --local >/dev/null
lfs="https://github.com/$VERCEL_GIT_REPO_OWNER/$VERCEL_GIT_REPO_SLUG.git/info/lfs"
git -c lfs.url="$lfs" lfs pull --include="assets/runtime/**,assets/third-party/effects/**"

bun install --cwd web
rustup target add wasm32-unknown-unknown
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
