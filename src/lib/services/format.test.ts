import { describe, it, expect } from "vitest";
import { formatSize, formatDate, getFileColor } from "./format";

describe("formatSize", () => {
  it("returns dash for zero", () => {
    expect(formatSize(0)).toBe("—");
  });

  it("formats bytes", () => {
    expect(formatSize(500)).toBe("500 B");
  });

  it("formats kilobytes", () => {
    expect(formatSize(1024)).toBe("1.0 KB");
    expect(formatSize(1536)).toBe("1.5 KB");
  });

  it("formats megabytes", () => {
    expect(formatSize(1048576)).toBe("1.0 MB");
  });

  it("formats gigabytes", () => {
    expect(formatSize(1073741824)).toBe("1.0 GB");
  });
});

describe("formatDate", () => {
  it("returns dash for null", () => {
    expect(formatDate(null)).toBe("—");
  });

  it("formats ISO date", () => {
    const result = formatDate("2024-03-15T10:30:00Z");
    expect(result).toContain("2024");
    expect(result).toContain("15");
  });
});

describe("getFileColor", () => {
  it("returns folder color for directories", () => {
    expect(getFileColor(null, true)).toBe("var(--file-folder)");
  });

  it("returns code color for ts files", () => {
    expect(getFileColor("ts", false)).toBe("var(--file-code)");
  });

  it("returns image color for png files", () => {
    expect(getFileColor("png", false)).toBe("var(--file-image)");
  });

  it("returns default for unknown extensions", () => {
    expect(getFileColor("xyz", false)).toBe("var(--file-default)");
  });

  it("returns default for null extension", () => {
    expect(getFileColor(null, false)).toBe("var(--file-default)");
  });
});
