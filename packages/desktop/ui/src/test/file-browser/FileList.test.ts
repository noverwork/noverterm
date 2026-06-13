import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

import type { FileEntry } from "$lib/types/sftp.js";

import FileList from "$lib/components/file-browser/FileList.svelte";

const FIXED_TIMESTAMP = 1_700_000_000;

function buildEntry(overrides: Partial<FileEntry> = {}): FileEntry {
  return {
    name: "file.txt",
    size: 0,
    modified: null,
    file_type: "File",
    ...overrides,
  };
}

function renderFileList(options: {
  files?: FileEntry[];
  selected?: FileEntry | null;
  loading?: boolean;
  onSelect?: (entry: FileEntry) => void;
  onNavigate?: (entry: FileEntry) => void;
  scrollKey?: string;
} = {}) {
  const onSelect = options.onSelect ?? vi.fn();
  const onNavigate = options.onNavigate ?? vi.fn();
  const result = render(FileList, {
    props: {
      files: options.files ?? [],
      selected: options.selected ?? null,
      loading: options.loading ?? false,
      scrollKey: options.scrollKey,
      onSelect,
      onNavigate,
    },
  });
  return { ...result, onSelect, onNavigate };
}

describe("FileList", () => {
  it("renders empty state when no files", () => {
    renderFileList();

    expect(screen.getByTestId("file-list-empty")).toBeTruthy();
    expect(screen.getByTestId("file-list-empty").textContent).toContain("No files");
    expect(screen.queryAllByTestId("file-row")).toHaveLength(0);
  });

  it("renders loading state when loading is true", () => {
    renderFileList({ loading: true, files: [] });

    expect(screen.getByTestId("file-list-loading")).toBeTruthy();
    expect(screen.queryByTestId("file-list-empty")).toBeNull();
  });

  it("keeps rows mounted while refreshing an existing directory", () => {
    const files = [buildEntry({ name: "alpha.txt" })];

    renderFileList({ files, loading: true, scrollKey: "/tmp" });

    expect(screen.getByTestId("file-list-rows")).toBeTruthy();
    expect(screen.getByTestId("file-list-refreshing")).toBeTruthy();
    expect(screen.queryByTestId("file-list-loading")).toBeNull();
    expect(screen.getByText("alpha.txt")).toBeTruthy();
  });

  it("preserves scroll position after refreshing the same directory", async () => {
    const files = Array.from({ length: 40 }, (_, index) =>
      buildEntry({ name: `file-${index.toString().padStart(2, "0")}.txt` }),
    );
    const { rerender } = renderFileList({ files, scrollKey: "/tmp" });
    const scrollContainer = screen.getByTestId("file-list-scroll") as HTMLDivElement;
    scrollContainer.scrollTop = 180;
    await fireEvent.scroll(scrollContainer);

    await rerender({
      files,
      selected: null,
      loading: true,
      scrollKey: "/tmp",
      onSelect: vi.fn(),
      onNavigate: vi.fn(),
    });
    await rerender({
      files: [...files, buildEntry({ name: "new-file.txt" })],
      selected: null,
      loading: false,
      scrollKey: "/tmp",
      onSelect: vi.fn(),
      onNavigate: vi.fn(),
    });

    await waitFor(() => {
      expect(scrollContainer.scrollTop).toBe(180);
    });
  });

  it("resets scroll position when navigating to a different directory", async () => {
    const files = Array.from({ length: 20 }, (_, index) =>
      buildEntry({ name: `file-${index.toString().padStart(2, "0")}.txt` }),
    );
    const { rerender } = renderFileList({ files, scrollKey: "/tmp" });
    const scrollContainer = screen.getByTestId("file-list-scroll") as HTMLDivElement;
    scrollContainer.scrollTop = 120;
    await fireEvent.scroll(scrollContainer);

    await rerender({
      files,
      selected: null,
      loading: false,
      scrollKey: "/tmp/subdir",
      onSelect: vi.fn(),
      onNavigate: vi.fn(),
    });

    await waitFor(() => {
      expect(scrollContainer.scrollTop).toBe(0);
    });
  });

  it("renders file list with mixed types", () => {
    const files: FileEntry[] = [
      buildEntry({ name: "notes.md", size: 2048, file_type: "File", modified: FIXED_TIMESTAMP }),
      buildEntry({ name: "projects", size: 0, file_type: "Dir", modified: FIXED_TIMESTAMP }),
      buildEntry({ name: "link-to-bin", size: 0, file_type: "Symlink", modified: null }),
      buildEntry({ name: "weird", size: 0, file_type: "Other", modified: null }),
    ];

    renderFileList({ files });

    const rows = screen.getAllByTestId("file-row");
    expect(rows).toHaveLength(4);
    expect(screen.getByText("notes.md")).toBeTruthy();
    expect(screen.getByText("projects")).toBeTruthy();
    expect(screen.getByText("link-to-bin")).toBeTruthy();
    expect(screen.getByText("weird")).toBeTruthy();
  });

  it("truncates long file names and keeps the full name as a tooltip", () => {
    const longName = "this-is-a-very-long-sftp-filename-that-needs-truncation.txt";

    renderFileList({ files: [buildEntry({ name: longName })] });

    expect(screen.getByText("this-is-a-very-long-sftp-filename-tha...")).toBeTruthy();
    expect(screen.getByTitle(longName)).toBeTruthy();
  });

  it("formats size human-readable", () => {
    const files: FileEntry[] = [
      buildEntry({ name: "tiny.txt", size: 512 }),
      buildEntry({ name: "one-kb.txt", size: 1024 }),
      buildEntry({ name: "one-and-half.txt", size: 1536 }),
      buildEntry({ name: "one-mb.bin", size: 1024 * 1024 }),
      buildEntry({ name: "one-gb.bin", size: 1024 * 1024 * 1024 }),
    ];

    renderFileList({ files });

    expect(screen.getByText("512 B")).toBeTruthy();
    expect(screen.getByText("1.0 KB")).toBeTruthy();
    expect(screen.getByText("1.5 KB")).toBeTruthy();
    expect(screen.getByText("1.0 MB")).toBeTruthy();
    expect(screen.getByText("1.0 GB")).toBeTruthy();
  });

  it("formats modified date and shows em-dash for null timestamps", () => {
    const files: FileEntry[] = [
      buildEntry({ name: "dated.txt", size: 1, modified: FIXED_TIMESTAMP }),
      buildEntry({ name: "undated.txt", size: 1, modified: null }),
    ];

    renderFileList({ files });

    const allRows = screen.getAllByTestId("file-row");
    const datedRow = allRows.find((row) => row.getAttribute("data-file-name") === "dated.txt");
    const undatedRow = allRows.find(
      (row) => row.getAttribute("data-file-name") === "undated.txt",
    );

    expect(datedRow?.textContent).toContain("2023");
    expect(datedRow?.textContent).toMatch(/11\/1[45]\/2023/);
    expect(undatedRow?.textContent).toContain("—");
  });

  it("highlights the selected row", () => {
    const files: FileEntry[] = [
      buildEntry({ name: "first.txt", size: 1 }),
      buildEntry({ name: "second.txt", size: 1 }),
    ];
    const selected = files[1]!;

    renderFileList({ files, selected });

    const allRows = screen.getAllByTestId("file-row");
    const firstRow = allRows.find((row) => row.getAttribute("data-file-name") === "first.txt");
    const secondRow = allRows.find((row) => row.getAttribute("data-file-name") === "second.txt");

    expect(firstRow?.getAttribute("data-selected")).toBe("false");
    expect(secondRow?.getAttribute("data-selected")).toBe("true");
    expect(secondRow?.className).toContain("bg-cyan-300/15");
  });

  it("emits onSelect when a row is clicked", async () => {
    const target = buildEntry({ name: "alpha.txt", size: 100 });
    const { onSelect } = renderFileList({ files: [target] });

    await fireEvent.click(screen.getByTestId("file-row"));

    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith(target);
  });

  it("emits onNavigate when a directory row is double-clicked", async () => {
    const directory = buildEntry({ name: "docs", file_type: "Dir" });
    const { onNavigate } = renderFileList({ files: [directory] });

    await fireEvent.dblClick(screen.getByTestId("file-row"));

    expect(onNavigate).toHaveBeenCalledTimes(1);
    expect(onNavigate).toHaveBeenCalledWith(directory);
  });

  it("does not emit onNavigate when double-clicking a file row", async () => {
    const file = buildEntry({ name: "report.pdf", file_type: "File" });
    const { onNavigate } = renderFileList({ files: [file] });

    await fireEvent.dblClick(screen.getByTestId("file-row"));

    expect(onNavigate).not.toHaveBeenCalled();
  });

  it("emits onNavigate when clicking the name of a directory entry", async () => {
    const directory = buildEntry({ name: "src", file_type: "Dir" });
    const { onSelect, onNavigate } = renderFileList({ files: [directory] });

    const nameButton = screen.getByText("src");
    await fireEvent.click(nameButton);

    expect(onNavigate).toHaveBeenCalledWith(directory);
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("emits onSelect when clicking the name of a file entry", async () => {
    const file = buildEntry({ name: "readme.md", file_type: "File" });
    const { onSelect, onNavigate } = renderFileList({ files: [file] });

    await fireEvent.click(screen.getByText("readme.md"));

    expect(onSelect).toHaveBeenCalledWith(file);
    expect(onNavigate).not.toHaveBeenCalled();
  });

  it("sorts directories before files", () => {
    const files: FileEntry[] = [
      buildEntry({ name: "zebra.txt", file_type: "File" }),
      buildEntry({ name: "archive", file_type: "Dir" }),
      buildEntry({ name: "alpha.txt", file_type: "File" }),
      buildEntry({ name: "beta", file_type: "Dir" }),
    ];

    renderFileList({ files });

    const rows = screen.getAllByTestId("file-row");
    const namesInOrder = rows.map((row) => row.getAttribute("data-file-name"));

    expect(namesInOrder).toEqual(["archive", "beta", "alpha.txt", "zebra.txt"]);
  });
});
