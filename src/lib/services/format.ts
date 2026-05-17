export function formatSize(bytes: number): string {
  if (bytes <= 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const size = bytes / Math.pow(1024, i);
  return `${size.toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

export function formatDate(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return d.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

export function formatDuration(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function formatBitrate(kbps: number): string {
  return `${kbps} kbps`;
}

export function formatSampleRate(hz: number): string {
  return `${(hz / 1000).toFixed(1)} kHz`;
}

export function getFileColor(extension: string | null, isDir: boolean): string {
  if (isDir) return "var(--file-folder)";
  if (!extension) return "var(--file-default)";

  const ext = extension.toLowerCase();

  const imageExts = ["png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "ico", "tiff"];
  const codeExts = ["ts", "js", "tsx", "jsx", "rs", "py", "go", "c", "cpp", "h", "java", "rb", "php", "css", "html", "svelte", "vue"];
  const docExts = ["pdf", "doc", "docx", "txt", "md", "rtf", "odt", "xls", "xlsx", "ppt", "pptx", "csv"];
  const archiveExts = ["zip", "tar", "gz", "rar", "7z", "bz2", "xz", "dmg", "iso"];
  const mediaExts = ["mp3", "mp4", "wav", "flac", "ogg", "avi", "mkv", "mov", "wmv", "m4a"];

  if (imageExts.includes(ext)) return "var(--file-image)";
  if (codeExts.includes(ext)) return "var(--file-code)";
  if (docExts.includes(ext)) return "var(--file-doc)";
  if (archiveExts.includes(ext)) return "var(--file-archive)";
  if (mediaExts.includes(ext)) return "var(--file-media)";

  return "var(--file-default)";
}
