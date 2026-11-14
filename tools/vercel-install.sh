#!/bin/sh
# Vercel's install step (vercel.json). The project's Git LFS setting is off, so
# the clone carries pointer files; the site serves only assets/runtime, so fetch
# just those objects rather than every source, reference and evidence file.
set -eu

# Vercel's clone has no remote and no LFS filter: install the filter so the
# pull writes files, and name the (public) repository's LFS store.
git lfs install --local >/dev/null
lfs="https://github.com/$VERCEL_GIT_REPO_OWNER/$VERCEL_GIT_REPO_SLUG.git/info/lfs"
git -c lfs.url="$lfs" lfs pull --include="assets/runtime/**"
# A pointer left behind would deploy as broken media; fail the build instead.
if git lfs ls-files --include="assets/runtime/**" | grep -q '^[0-9a-f]* - '; then
  echo "vercel-install: assets/runtime still has LFS pointer files" >&2
  exit 1
fi

bun install --cwd web
rustup target add wasm32-unknown-unknown
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
