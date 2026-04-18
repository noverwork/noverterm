import { defineConfig } from "vitest/config";
import { sveltekit } from "@sveltejs/kit/vite";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(__dirname);

export default defineConfig({
  plugins: [sveltekit()],
  resolve: {
    alias: {
      "$bindings": path.resolve(projectRoot, "../../ui/src/bindings.ts"),
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/test/**/*.test.ts"],
  },
});
