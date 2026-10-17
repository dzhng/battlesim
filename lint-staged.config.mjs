const runWithoutStagedPaths = (command) => () => command;

// Each glob covers the same paths as the matching `bun run lint` / `fmt:check`
// step, so a commit is never refused for something those checks would pass.
export default {
  "{packages,apps,web/src}/**/*.{js,jsx,mjs,cjs,ts,tsx}": [
    "web/node_modules/.bin/oxfmt --write",
    "web/node_modules/.bin/eslint --config web/eslint.config.mjs --max-warnings=0",
    "web/node_modules/.bin/oxlint -c web/.oxlintrc.json",
  ],
  "{web/tests,web/scenes}/**/*.{js,jsx,mjs,cjs,ts,tsx}": ["web/node_modules/.bin/oxfmt --write"],
  "{web/*.{mjs,mts,ts},lint-staged.config.mjs}": ["web/node_modules/.bin/oxfmt --write"],
  "{packages,apps,web/src,web/tests,web/scenes}/**/*.{json,md,css,scss}": [
    "web/node_modules/.bin/oxfmt --write",
  ],
  "*.rs": [
    runWithoutStagedPaths("cargo fmt --all"),
    runWithoutStagedPaths("cargo clippy --workspace --all-targets -- -D warnings"),
  ],
};
