import { describe, it, expect } from "vitest";
import { formatSize, formatDate, getFileColor } from "./format";

describe("formatSize", () => {
  it("distinguishes an empty file from a missing value", () => {
    expect(formatSize(0)).toBe("0 B");
    expect(formatSize(null)).toBe("—");
    expect(formatSize(undefined)).toBe("—");
    expect(formatSize(-1)).toBe("—");
  });

  it("formats bytes", () => {
    expect(formatSize(500)).toBe("500 B");
  });

  it("matches Finder's decimal units", () => {
    expect(formatSize(1000)).toBe("1.0 KB");
    expect(formatSize(1500)).toBe("1.5 KB");
    expect(formatSize(1_000_000)).toBe("1.0 MB");
    expect(formatSize(1_000_000_000)).toBe("1.0 GB");
    expect(formatSize(2_000_000_000_000)).toBe("2.0 TB");
  });

  it("rolls up to the next unit instead of printing 1000.0", () => {
    expect(formatSize(999)).toBe("999 B");
    expect(formatSize(999_999)).toBe("1.0 MB");
    expect(formatSize(999_999_999)).toBe("1.0 GB");
  });

  it("caps at the largest unit", () => {
    expect(formatSize(5_000_000_000_000_000)).toBe("5000.0 TB");
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
