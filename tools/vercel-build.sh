#!/bin/sh
# Vercel's build step (vercel.json): build the site, then refuse to deploy it if
# any of it is a Git LFS pointer instead of media. A pointer reaches the site as
# a file (public assets, bundled images) or, being tiny, inlined into a bundle
# as a base64 data URL; the second pattern is "version https://git-lfs." in
# base64. vercel-install.sh fetches what the site needs today; this catches the
# next thing it imports.
set -eu

bun run build
pointers=$(grep -rlE '^version https://git-lfs\.github\.com/spec/|dmVyc2lvbiBodHRwczovL2dpdC1sZnMu' web/dist || true)
if [ -n "$pointers" ]; then
  echo "vercel-build: the site carries Git LFS pointers; add their paths to tools/vercel-install.sh:" >&2
  echo "$pointers" >&2
  exit 1
fi
