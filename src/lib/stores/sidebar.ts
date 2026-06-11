import { writable } from "svelte/store";
import type { BreadcrumbItem } from "../types";

const STORAGE_KEY = "kathaloq-sidebar";

interface SidebarState {
  expandedCatalogIds: number[];
  activeCatalogId: number | null;
  // Full ancestor path so a reload restores breadcrumb + up-nav, not one segment.
  selectedFolderPath: BreadcrumbItem[];
}

const EMPTY: SidebarState = {
  expandedCatalogIds: [],
  activeCatalogId: null,
  selectedFolderPath: [],
};

function isBreadcrumbPath(v: unknown): v is BreadcrumbItem[] {
  return Array.isArray(v) && v.every((x) => x && typeof x === "object" && "id" in x && "name" in x);
}

function getInitial(): SidebarState {
  if (typeof window === "undefined") {
    return { ...EMPTY };
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        expandedCatalogIds: Array.isArray(parsed.expandedCatalogIds)
          ? parsed.expandedCatalogIds
          : [],
        activeCatalogId: typeof parsed.activeCatalogId === "number" ? parsed.activeCatalogId : null,
        selectedFolderPath: isBreadcrumbPath(parsed.selectedFolderPath)
          ? parsed.selectedFolderPath
          : [],
      };
    }
  } catch (e) {
    console.error(e);
  }
  return { ...EMPTY };
}

const initial = getInitial();

export const sidebarState = writable<SidebarState>(initial);

sidebarState.subscribe((value) => {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch (e) {
    console.error(e);
  }
});
