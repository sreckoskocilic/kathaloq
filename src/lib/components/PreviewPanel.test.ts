/// <reference types="node" />
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// Regression guard for the UI-freeze bug: PreviewPanel/InfoModal briefly hold
// mediaTags/folderStats/bulkStats as null on every selection change. If the
// template dereferences the {@const} aliases (`tags`/`stats`) without optional
// chaining, Svelte's inner {#if tags.title} effect re-runs with null mid-flush
// and throws "null is not an object", which wedges the render loop and freezes
// every click. Optional chaining makes that throw impossible at the language
// level. This asserts the aliases are never dereferenced raw, in either file.
//
// A render test was tried first but jsdom's effect timing doesn't reproduce the
// webview's flush ordering, so it passed even with the bug present — useless as
// a guard. This source check is deterministic.

const FILES = ["./PreviewPanel.svelte", "./InfoModal.svelte"];

// Matches a raw `tags.` or `stats.` deref. `tags?.`/`stats?.` won't match (the
// `?` sits between the name and the dot). The `@const tags = mediaTags` lines
// have no dot after the alias, and `bulkMediaTags.`/`bulkStats.` use capital
// letters, so the word-boundary lowercase pattern skips them.
const RAW_DEREF = /\b(?:tags|stats)\.[a-z]/g;

describe("preview/info null-safety", () => {
  for (const rel of FILES) {
    it(`${rel} only dereferences media/stats aliases via optional chaining`, () => {
      const src = readFileSync(fileURLToPath(new URL(rel, import.meta.url)), "utf8");
      const offenders = src.match(RAW_DEREF) ?? [];
      expect(offenders, `raw deref(s) found: ${offenders.join(", ")} — use ?.`).toEqual([]);
    });
  }
});
