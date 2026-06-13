import { describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";

import type { TransferProgress as TransferProgressType } from "$lib/types/sftp.js";
import TransferProgress from "$lib/components/file-browser/TransferProgress.svelte";

function makeTransfer(
  id: string,
  bytes_transferred: number,
  total_bytes: number,
  speed_bps: number,
  direction: TransferProgressType["direction"] = "Upload",
): TransferProgressType {
  return {
    transfer_id: id,
    bytes_transferred,
    total_bytes,
    speed_bps,
    direction,
  };
}

describe("TransferProgress", () => {
  it("renders no transfers when map is empty", () => {
    const { container } = render(TransferProgress, {
      transfers: new Map(),
      onCancel: vi.fn(),
    });

    expect(container.querySelector('[role="region"]')).toBeNull();
    expect(container.querySelectorAll('[data-testid="transfer-row"]')).toHaveLength(0);
  });

  it("renders single transfer with progress bar", () => {
    const transfers = new Map<string, TransferProgressType>([
      ["t-1", makeTransfer("t-1", 250, 1000, 512)],
    ]);

    const { container, getByTestId } = render(TransferProgress, {
      transfers,
      onCancel: vi.fn(),
    });

    expect(getByTestId("transfer-status-bar")).toBeTruthy();
    expect(getByTestId("transfer-count").textContent?.trim()).toBe("1 active");

    const rows = container.querySelectorAll('[data-testid="transfer-row"]');
    expect(rows).toHaveLength(1);
    expect(rows[0]?.getAttribute("data-transfer-id")).toBe("t-1");

    const bar = getByTestId("transfer-progress-bar") as HTMLElement;
    expect(bar.getAttribute("style")).toContain("width: 25%");
    expect(bar.getAttribute("aria-valuenow")).toBe("25");
    expect(bar.getAttribute("aria-valuemin")).toBe("0");
    expect(bar.getAttribute("aria-valuemax")).toBe("100");

    expect(container.textContent).toContain("25%");
    expect(container.textContent).toContain("Uploading t-1");
  });

  it("formats speed human-readable", () => {
    const transfers = new Map<string, TransferProgressType>([
      ["bytes", makeTransfer("bytes", 0, 1, 512)],
      ["kb", makeTransfer("kb", 0, 1, 2048)],
      ["mb", makeTransfer("mb", 0, 1, 5 * 1024 * 1024)],
    ]);

    const { getAllByTestId } = render(TransferProgress, {
      transfers,
      onCancel: vi.fn(),
    });

    const speedNodes = getAllByTestId("transfer-speed");
    expect(speedNodes[0]?.textContent?.trim()).toBe("512 B/s");
    expect(speedNodes[1]?.textContent?.trim()).toBe("2.0 KB/s");
    expect(speedNodes[2]?.textContent?.trim()).toBe("5.0 MB/s");
  });

  it("formats transferred and total bytes in the status row", () => {
    const transfers = new Map<string, TransferProgressType>([
      ["bytes", makeTransfer("bytes", 512, 1024, 512)],
      ["mb", makeTransfer("mb", 5 * 1024 * 1024, 10 * 1024 * 1024, 1024)],
    ]);

    const { getAllByTestId } = render(TransferProgress, {
      transfers,
      onCancel: vi.fn(),
    });

    const sizeNodes = getAllByTestId("transfer-size");
    expect(sizeNodes[0]?.textContent?.trim()).toBe("512 B / 1.0 KB");
    expect(sizeNodes[1]?.textContent?.trim()).toBe("5.0 MB / 10.0 MB");
  });

  it("calculates percentage correctly", () => {
    const transfers = new Map<string, TransferProgressType>([
      ["half", makeTransfer("half", 500, 1000, 1024)],
      ["complete", makeTransfer("complete", 1000, 1000, 1024)],
      ["zero", makeTransfer("zero", 0, 1000, 1024)],
    ]);

    const { getAllByTestId } = render(TransferProgress, {
      transfers,
      onCancel: vi.fn(),
    });

    const bars = getAllByTestId("transfer-progress-bar");
    expect(bars[0]?.getAttribute("style")).toContain("width: 50%");
    expect(bars[0]?.getAttribute("aria-valuenow")).toBe("50");
    expect(bars[1]?.getAttribute("style")).toContain("width: 100%");
    expect(bars[1]?.getAttribute("aria-valuenow")).toBe("100");
    expect(bars[2]?.getAttribute("style")).toContain("width: 0%");
    expect(bars[2]?.getAttribute("aria-valuenow")).toBe("0");
  });

  it("emits onCancel when button clicked", async () => {
    const onCancel = vi.fn();
    const transfers = new Map<string, TransferProgressType>([
      ["c-1", makeTransfer("c-1", 100, 1000, 1024)],
      ["c-2", makeTransfer("c-2", 200, 1000, 1024)],
    ]);

    const { getAllByTestId } = render(TransferProgress, {
      transfers,
      onCancel,
    });

    const buttons = getAllByTestId("transfer-cancel");
    expect(buttons).toHaveLength(2);

    buttons[0]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    buttons[1]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));

    expect(onCancel).toHaveBeenCalledTimes(2);
    expect(onCancel).toHaveBeenNthCalledWith(1, "c-1");
    expect(onCancel).toHaveBeenNthCalledWith(2, "c-2");
  });

  it("renders multiple transfers", () => {
    const transfers = new Map<string, TransferProgressType>([
      ["a", makeTransfer("a", 100, 1000, 1024, "Upload")],
      ["b", makeTransfer("b", 200, 1000, 2048, "Download")],
      ["c", makeTransfer("c", 300, 1000, 4096, "Upload")],
    ]);

    const { container, getAllByTestId, getByTestId } = render(TransferProgress, {
      transfers,
      onCancel: vi.fn(),
    });

    const rows = container.querySelectorAll('[data-testid="transfer-row"]');
    expect(rows).toHaveLength(3);
    expect(getByTestId("transfer-count").textContent?.trim()).toBe("3 active");
    expect(getAllByTestId("transfer-progress-bar")).toHaveLength(3);
    expect(getAllByTestId("transfer-cancel")).toHaveLength(3);
    expect(container.textContent).toContain("Uploading");
    expect(container.textContent).toContain("Downloading");
  });
});
