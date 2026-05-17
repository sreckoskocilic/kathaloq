import js from "@eslint/js";
import tseslint from "typescript-eslint";
import svelte from "eslint-plugin-svelte";
import globals from "globals";

export default tseslint.config(
  {
    ignores: ["dist/", "src-tauri/", "node_modules/"],
  },
  {
    files: ["src/**/*.ts"],
    ...js.configs.recommended,
    languageOptions: {
      globals: {
        ...globals.browser,
      },
    },
  },
  ...tseslint.configs.recommended.map((config) => ({
    ...config,
    files: ["src/**/*.ts"],
  })),
  ...svelte.configs["flat/recommended"],
  {
    files: ["src/**/*.svelte", "src/**/*.svelte.ts", "src/**/*.svelte.js"],
    languageOptions: {
      globals: {
        ...globals.browser,
      },
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  }
);
