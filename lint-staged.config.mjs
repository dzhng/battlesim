const runWithoutStagedPaths = (command) => () => command;

// No glob reaches past the paths `bun run lint` and `fmt:check` cover, so the
// hook never refuses or rewrites a file those checks leave alone.
export default {
  "{packages,apps,web/src}/**/*.{js,jsx,mjs,cjs,ts,tsx}": [
    "web/node_modules/.bin/oxfmt --write",
    "web/node_modules/.bin/eslint --config web/eslint.config.mjs --max-warnings=0 --no-warn-ignored",
    "web/node_modules/.bin/oxlint -c web/.oxlintrc.json",
  ],
  "{web/tests,web/scenes}/**/*.{js,jsx,mjs,cjs,ts,tsx}": ["web/node_modules/.bin/oxfmt --write"],
  "{web/{scene,scene.test,browser,browser.test,asset,eslint.config}.mjs,web/browser.d.mts,web/{vite,vitest}.config.ts,lint-staged.config.mjs}":
    ["web/node_modules/.bin/oxfmt --write"],
  "{packages,apps,web/src,web/tests,web/scenes}/**/*.{json,md,css,scss}": [
    "web/node_modules/.bin/oxfmt --write",
  ],
  "*.rs": [
    runWithoutStagedPaths("cargo fmt --all"),
    runWithoutStagedPaths("cargo clippy --workspace --all-targets -- -D warnings"),
  ],
};
