import { fireEvent, render } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import DeleteConfirmDialog from "$lib/components/file-browser/DeleteConfirmDialog.svelte";

describe("DeleteConfirmDialog", () => {
  let onConfirm: () => void;
  let onCancel: () => void;

  beforeEach(() => {
    onConfirm = vi.fn();
    onCancel = vi.fn();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function renderDialog(open: boolean, itemName: string) {
    return render(DeleteConfirmDialog, {
      props: { open, itemName, onConfirm, onCancel },
    });
  }

  it("does not render when open is false", () => {
    const { queryByRole } = renderDialog(false, "secret.txt");
    expect(queryByRole("dialog")).toBeNull();
  });

  it("renders the warning, target name, and both buttons when open", () => {
    const { getByRole, getByText } = renderDialog(true, "secret.txt");

    expect(getByRole("dialog")).toBeTruthy();
    expect(getByText(/delete this item/i)).toBeTruthy();
    expect(getByText(/cannot be undone/i)).toBeTruthy();
    expect(getByText("secret.txt")).toBeTruthy();
    expect(getByRole("button", { name: /^delete$/i })).toBeTruthy();
    expect(getByRole("button", { name: /cancel/i })).toBeTruthy();
  });

  it("calls onConfirm when Delete button is clicked", async () => {
    const { getByRole } = renderDialog(true, "secret.txt");

    await fireEvent.click(getByRole("button", { name: /^delete$/i }));

    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("calls onCancel when Cancel button is clicked", async () => {
    const { getByRole } = renderDialog(true, "secret.txt");

    await fireEvent.click(getByRole("button", { name: /cancel/i }));

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("calls onCancel when Escape is pressed", async () => {
    const { getByRole } = renderDialog(true, "secret.txt");
    const dialog = getByRole("dialog");

    await fireEvent.keyDown(dialog, { key: "Escape" });

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("does not call onCancel for other keys", async () => {
    const { getByRole } = renderDialog(true, "secret.txt");
    const dialog = getByRole("dialog");

    await fireEvent.keyDown(dialog, { key: "Enter" });
    await fireEvent.keyDown(dialog, { key: " " });
    await fireEvent.keyDown(dialog, { key: "a" });

    expect(onCancel).not.toHaveBeenCalled();
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("calls onCancel when the backdrop is clicked", async () => {
    const { container, getByRole } = renderDialog(true, "secret.txt");
    const dialog = getByRole("dialog");
    const backdrop = dialog.parentElement;
    expect(backdrop).toBeTruthy();

    await fireEvent.click(backdrop as HTMLElement);

    expect(onCancel).toHaveBeenCalledTimes(1);
    void container;
  });

  it("does not call onCancel when clicking inside the dialog card", async () => {
    const { getByRole, getByText } = renderDialog(true, "secret.txt");

    await fireEvent.click(getByText("secret.txt"));
    await fireEvent.click(getByRole("button", { name: /^delete$/i }));

    expect(onCancel).not.toHaveBeenCalled();
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("updates the displayed target name when the itemName prop changes", async () => {
    const { rerender, getByText, queryByText } = renderDialog(true, "first.txt");
    expect(getByText("first.txt")).toBeTruthy();

    await rerender({ open: true, itemName: "second.txt", onConfirm, onCancel });
    expect(getByText("second.txt")).toBeTruthy();
    expect(queryByText("first.txt")).toBeNull();
  });
});
