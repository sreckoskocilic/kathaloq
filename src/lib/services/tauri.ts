import { invoke } from "@tauri-apps/api/core";
import type { Catalog, FileEntry, FolderStats, MediaTags, UpdatePreview } from "../types";

export async function startScan(path: string, name: string): Promise<number> {
  return invoke<number>("start_scan", { path, name });
}

export async function listCatalogs(): Promise<Catalog[]> {
  return invoke<Catalog[]>("list_catalogs");
}

export async function deleteCatalog(id: number): Promise<void> {
  return invoke<void>("delete_catalog", { id });
}

export async function getChildren(
  catalogId: number,
  parentId: number | null
): Promise<FileEntry[]> {
  return invoke<FileEntry[]>("get_children", { catalogId, parentId });
}

export async function searchFiles(
  catalogId: number,
  query: string
): Promise<FileEntry[]> {
  return invoke<FileEntry[]>("search_files", { catalogId, query });
}

export async function previewCatalogUpdate(catalogId: number): Promise<UpdatePreview> {
  return invoke<UpdatePreview>("preview_catalog_update", { catalogId });
}

export async function applyCatalogUpdate(catalogId: number): Promise<UpdatePreview> {
  return invoke<UpdatePreview>("apply_catalog_update", { catalogId });
}

export async function removeFileEntries(catalogId: number, ids: number[]): Promise<void> {
  return invoke<void>("remove_file_entries", { catalogId, ids });
}

export async function getFolderStats(catalogId: number, folderId: number): Promise<FolderStats> {
  return invoke<FolderStats>("get_folder_stats", { catalogId, folderId });
}

export async function getBulkStats(catalogId: number, ids: number[]): Promise<FolderStats> {
  return invoke<FolderStats>("get_bulk_stats", { catalogId, ids });
}

export async function getMediaTags(fileEntryId: number): Promise<MediaTags | null> {
  return invoke<MediaTags | null>("get_media_tags", { fileEntryId });
}

export async function getMediaTagsBulk(fileEntryIds: number[]): Promise<MediaTags[]> {
  return invoke<MediaTags[]>("get_media_tags_bulk", { fileEntryIds });
}
