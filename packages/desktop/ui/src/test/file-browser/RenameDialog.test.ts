import { fireEvent, render } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import RenameDialog from "$lib/components/file-browser/RenameDialog.svelte";

describe("RenameDialog", () => {
  let onConfirm: (newName: string) => void;
  let onCancel: () => void;

  beforeEach(() => {
    onConfirm = vi.fn();
    onCancel = vi.fn();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function renderDialog(
    open: boolean,
    currentName: string,
    overrides?: {
      onConfirm?: (newName: string) => void;
      onCancel?: () => void;
    },
  ) {
    return render(RenameDialog, {
      props: {
        open,
        currentName,
        onConfirm: overrides?.onConfirm ?? onConfirm,
        onCancel: overrides?.onCancel ?? onCancel,
      },
    });
  }

  it("does not render when open is false", () => {
    const { queryByRole } = renderDialog(false, "old.txt");
    expect(queryByRole("dialog")).toBeNull();
  });

  it("renders with input pre-filled and select-ready when open", async () => {
    const { getByRole, getByLabelText } = renderDialog(true, "old.txt");

    expect(getByRole("dialog")).toBeTruthy();
    const input = getByLabelText(/new name/i) as HTMLInputElement;
    expect(input.value).toBe("old.txt");
    expect(getByRole("button", { name: /rename/i })).toBeTruthy();
    expect(getByRole("button", { name: /cancel/i })).toBeTruthy();

    await Promise.resolve();
    expect(document.activeElement).toBe(input);
    expect(input.selectionStart).toBe(0);
    expect(input.selectionEnd).toBe("old.txt".length);
  });

  it("calls onConfirm with the new trimmed name when renamed", async () => {
    const { getByLabelText, getByRole } = renderDialog(true, "old.txt");
    const input = getByLabelText(/new name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "new.txt" } });
    await fireEvent.click(getByRole("button", { name: /rename/i }));

    expect(onConfirm).toHaveBeenCalledWith("new.txt");
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("rejects an empty name", async () => {
    const { getByLabelText, getByRole, getByText } = renderDialog(true, "old.txt");
    const input = getByLabelText(/new name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "   " } });
    await fireEvent.click(getByRole("button", { name: /rename/i }));

    expect(onConfirm).not.toHaveBeenCalled();
    expect(getByText(/name is required/i)).toBeTruthy();
  });

  it("rejects a name equal to the current name", async () => {
    const { getByLabelText, getByRole, getByText } = renderDialog(true, "old.txt");
    const input = getByLabelText(/new name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "old.txt" } });
    await fireEvent.click(getByRole("button", { name: /rename/i }));

    expect(onConfirm).not.toHaveBeenCalled();
    expect(getByText(/name is unchanged/i)).toBeTruthy();
  });

  it("rejects names containing slashes", async () => {
    const { getByLabelText, getByRole, getByText } = renderDialog(true, "old.txt");
    const input = getByLabelText(/new name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "new/name" } });
    await fireEvent.click(getByRole("button", { name: /rename/i }));

    expect(onConfirm).not.toHaveBeenCalled();
    expect(getByText(/invalid characters/i)).toBeTruthy();
  });

  it("rejects names containing backslashes, colons, or other forbidden characters", async () => {
    const { getByLabelText, getByRole } = renderDialog(true, "old.txt");
    const input = getByLabelText(/new name/i) as HTMLInputElement;

    for (const bad of ["a\\b", "a:b", "a*b", "a?b", "a\"b", "a<b"]) {
      await fireEvent.input(input, { target: { value: bad } });
      await fireEvent.click(getByRole("button", { name: /rename/i }));
      expect(onConfirm).not.toHaveBeenCalled();
    }
  });

  it("rejects names equal to '.' or '..'", async () => {
    const { getByLabelText, getByRole } = renderDialog(true, "old.txt");
    const input = getByLabelText(/new name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "." } });
    await fireEvent.click(getByRole("button", { name: /rename/i }));
    expect(onConfirm).not.toHaveBeenCalled();

    await fireEvent.input(input, { target: { value: ".." } });
    await fireEvent.click(getByRole("button", { name: /rename/i }));
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("calls onCancel when Cancel is clicked", async () => {
    const { getByRole } = renderDialog(true, "old.txt");
    await fireEvent.click(getByRole("button", { name: /cancel/i }));
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("calls onCancel when Escape is pressed", async () => {
    const { getByRole } = renderDialog(true, "old.txt");
    const dialog = getByRole("dialog");
    await fireEvent.keyDown(dialog, { key: "Escape" });
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("confirms when Enter is pressed with a changed valid name", async () => {
    const { getByLabelText } = renderDialog(true, "old.txt");
    const input = getByLabelText(/new name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "new.txt" } });
    await fireEvent.keyDown(input, { key: "Enter" });

    expect(onConfirm).toHaveBeenCalledWith("new.txt");
  });

  it("resets the input to the new currentName when the prop changes", async () => {
    const { rerender, getByLabelText } = renderDialog(true, "old.txt");
    const input = getByLabelText(/new name/i) as HTMLInputElement;
    expect(input.value).toBe("old.txt");

    await rerender({ open: false, currentName: "old.txt", onConfirm, onCancel });
    await rerender({ open: true, currentName: "another.txt", onConfirm, onCancel });

    const refreshed = getByLabelText(/new name/i) as HTMLInputElement;
    expect(refreshed.value).toBe("another.txt");
  });
});
