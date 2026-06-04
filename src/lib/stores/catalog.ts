import { writable, derived } from "svelte/store";
import type { Catalog, FileEntry, BreadcrumbItem, MediaFilter } from "../types";

export const catalogs = writable<Catalog[]>([]);
export const activeCatalogId = writable<number | null>(null);
export const currentFiles = writable<FileEntry[]>([]);
export const breadcrumbs = writable<BreadcrumbItem[]>([]);
export const isLoading = writable(false);
export const searchQuery = writable("");
export const mediaFilter = writable<MediaFilter>(null);

// Bumped on any backend mutation (add/update/remove/delete) so cached views —
// the sidebar folder tree (Sidebar.rootFolders + TreeNode.loaded) — invalidate.
export const catalogVersion = writable(0);

export function bumpCatalogVersion() {
  catalogVersion.update((v) => v + 1);
}

export const activeCatalog = derived(
  [catalogs, activeCatalogId],
  ([$catalogs, $id]) => $catalogs.find((c) => c.id === $id) ?? null
);
