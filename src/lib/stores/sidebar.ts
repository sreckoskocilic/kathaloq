import { writable } from "svelte/store";

const STORAGE_KEY = "kathaloq-sidebar";

interface SidebarState {
  expandedCatalogIds: number[];
  activeCatalogId: number | null;
  selectedFolderId: number | null;
}

function getInitial(): SidebarState {
  if (typeof window === "undefined") {
    return { expandedCatalogIds: [], activeCatalogId: null, selectedFolderId: null };
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        expandedCatalogIds: Array.isArray(parsed.expandedCatalogIds) ? parsed.expandedCatalogIds : [],
        activeCatalogId: typeof parsed.activeCatalogId === "number" ? parsed.activeCatalogId : null,
        selectedFolderId: typeof parsed.selectedFolderId === "number" ? parsed.selectedFolderId : null,
      };
    }
  } catch { /* */ }
  return { expandedCatalogIds: [], activeCatalogId: null, selectedFolderId: null };
}

const initial = getInitial();

export const sidebarState = writable<SidebarState>(initial);

sidebarState.subscribe((value) => {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch { /* */ }
});
