import tsParser from "@typescript-eslint/parser";
import reactHooks from "eslint-plugin-react-hooks";

export default [
  {
    ignores: ["**/node_modules/**", "**/src/wasm/**", "**/dist/**"],
  },
  {
    files: ["**/*.{js,mjs,cjs,ts,tsx}"],
    languageOptions: {
      parser: tsParser,
      ecmaVersion: "latest",
      sourceType: "module",
    },
    plugins: {
      "react-hooks": reactHooks,
    },
    rules: {
      complexity: ["error", { max: 20, variant: "classic" }],
    },
  },
  {
    files: ["apps/battle-lab/src/routes/workbench.tsx"],
    linterOptions: { reportUnusedDisableDirectives: "off" },
  },
];
