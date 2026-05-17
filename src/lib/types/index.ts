export interface Catalog {
  id: number;
  name: string;
  root_path: string;
  scanned_at: string;
  total_files: number;
  total_size: number;
}

export interface FileEntry {
  id: number;
  catalog_id: number;
  parent_id: number | null;
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: string | null;
  extension: string | null;
}

export type Theme = "obsidian" | "ember" | "slate" | "terminal";

export type SortField = "name" | "size" | "modified" | "extension";
export type SortDirection = "asc" | "desc";

export interface BreadcrumbItem {
  id: number | null;
  name: string;
}

export interface ColumnConfig {
  id: string;
  label: string;
  visible: boolean;
  width: string;
}

export interface TreeNode {
  entry: FileEntry;
  children: TreeNode[];
  expanded: boolean;
  loaded: boolean;
}

export interface AppSettings {
  columns: ColumnConfig[];
  theme: Theme;
}

export interface MediaTags {
  id: number;
  file_entry_id: number;
  duration_secs: number | null;
  bitrate: number | null;
  sample_rate: number | null;
  channels: number | null;
  title: string | null;
  artist: string | null;
  album: string | null;
  genre: string | null;
  year: number | null;
  track_number: number | null;
}

export interface FolderStats {
  file_count: number;
  folder_count: number;
  total_size: number;
}

export interface UpdatePreview {
  added: number;
  updated: number;
  deleted_files: number;
  deleted_folders: number;
  unchanged: number;
  tags_to_backfill: number;
}

export const DEFAULT_COLUMNS: ColumnConfig[] = [
  { id: "name", label: "Name", visible: true, width: "flex" },
  { id: "extension", label: "Type", visible: true, width: "80px" },
  { id: "size", label: "Size", visible: true, width: "90px" },
  { id: "modified", label: "Modified", visible: true, width: "150px" },
];
