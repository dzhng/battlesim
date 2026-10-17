const runWithoutStagedPaths = (command) => () => command;

export default {
  "*.{js,jsx,mjs,cjs,ts,tsx}": [
    "web/node_modules/.bin/oxfmt --write",
    "web/node_modules/.bin/eslint --config web/eslint.config.mjs --max-warnings=0",
    "web/node_modules/.bin/oxlint -c web/.oxlintrc.json",
  ],
  "*.{json,md,css,scss}": ["web/node_modules/.bin/oxfmt --write"],
  "*.rs": [
    runWithoutStagedPaths("cargo fmt --all"),
    runWithoutStagedPaths("cargo clippy --workspace --all-targets -- -D warnings"),
  ],
};
