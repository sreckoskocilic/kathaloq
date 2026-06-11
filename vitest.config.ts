import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  test: {
    environment: "jsdom",
    include: ["src/**/*.{test,spec}.{ts,js}"],
    globals: true,
    // Pin TZ so formatDate's local-time rendering is deterministic across machines
    // (off-UTC runners would roll a UTC timestamp into the previous/next day).
    env: { TZ: "UTC" },
  },
});
