const runWithoutStagedPaths = (command) => () => command;
// Staged ignored files can leave the formatter with no targets.
const FORMAT = "web/node_modules/.bin/oxfmt --write --no-error-on-unmatched-pattern";

// No formatter or linter glob reaches past the paths `bun run lint` and
// `fmt:check` cover, so the hook never rewrites a file those checks leave
// alone. The size ratchet sees every staged file and decides what is source.
export default {
  "*": "bun .agents/skills/ratchet-files/scripts/check-file-size.mjs",
  "{packages,apps,web/src}/**/*.{js,jsx,mjs,cjs,ts,tsx}": [
    FORMAT,
    "web/node_modules/.bin/eslint --config web/eslint.config.mjs --max-warnings=0 --no-warn-ignored",
    "web/node_modules/.bin/oxlint -c web/.oxlintrc.json",
  ],
  "{web/tests,web/scenes}/**/*.{js,jsx,mjs,cjs,ts,tsx}": [FORMAT],
  "{web/{scene,scene.test,browser,browser.test,asset,asset-render,eslint.config}.mjs,web/browser.d.mts,web/{vite,vitest}.config.ts,lint-staged.config.mjs}":
    [FORMAT],
  "{packages,apps,web/src,web/tests,web/scenes}/**/*.{json,md,css,scss}": [FORMAT],
  "*.rs": [
    runWithoutStagedPaths("cargo fmt --all"),
    runWithoutStagedPaths("cargo clippy --workspace --all-targets -- -D warnings"),
  ],
};
