import tsParser from "@typescript-eslint/parser";

export default [
  {
    ignores: ["**/node_modules/**", "**/src/wasm/**", "**/dist/**"],
  },
  {
    files: ["**/*.{js,jsx,mjs,cjs,ts,tsx}"],
    languageOptions: {
      parser: tsParser,
      ecmaVersion: "latest",
      sourceType: "module",
    },
    linterOptions: { reportUnusedDisableDirectives: "error" },
    rules: {
      complexity: ["error", { max: 40, variant: "classic" }],
    },
  },
];
