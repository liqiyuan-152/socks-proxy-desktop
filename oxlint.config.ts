import { defineConfig } from "oxlint";

export default defineConfig({
  categories: {
    correctness: "error",
    perf: "error",
    suspicious: "error",
  },
  ignorePatterns: ["dist", "node_modules", "src-tauri/target"],
  plugins: ["react", "typescript"],
  rules: {
    "no-console": "off",
    "react/react-in-jsx-scope": "off",
  },
});
