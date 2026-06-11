import { fireEvent, render } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import CreateFolderDialog from "$lib/components/file-browser/CreateFolderDialog.svelte";

describe("CreateFolderDialog", () => {
  let onConfirm: (name: string) => void;
  let onCancel: () => void;

  beforeEach(() => {
    onConfirm = vi.fn();
    onCancel = vi.fn();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function renderDialog(open: boolean) {
    return render(CreateFolderDialog, {
      props: { open, onConfirm, onCancel },
    });
  }

  it("does not render when open is false", () => {
    const { queryByRole } = renderDialog(false);
    expect(queryByRole("dialog")).toBeNull();
  });

  it("renders the dialog with input and buttons when open", () => {
    const { getByRole, getByLabelText } = renderDialog(true);
    expect(getByRole("dialog")).toBeTruthy();
    expect(getByLabelText(/folder name/i)).toBeTruthy();
    expect(getByRole("button", { name: /create/i })).toBeTruthy();
    expect(getByRole("button", { name: /cancel/i })).toBeTruthy();
  });

  it("calls onConfirm with the trimmed name when Create is clicked with a valid name", async () => {
    const { getByLabelText, getByRole } = renderDialog(true);
    const input = getByLabelText(/folder name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "my-folder" } });
    await fireEvent.click(getByRole("button", { name: /create/i }));

    expect(onConfirm).toHaveBeenCalledWith("my-folder");
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("trims whitespace from the name before confirming", async () => {
    const { getByLabelText, getByRole } = renderDialog(true);
    const input = getByLabelText(/folder name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "  spaced  " } });
    await fireEvent.click(getByRole("button", { name: /create/i }));

    expect(onConfirm).toHaveBeenCalledWith("spaced");
  });

  it("shows an error and does not confirm when name is empty", async () => {
    const { getByLabelText, getByRole, getByText } = renderDialog(true);
    const input = getByLabelText(/folder name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "   " } });
    await fireEvent.click(getByRole("button", { name: /create/i }));

    expect(onConfirm).not.toHaveBeenCalled();
    expect(getByText(/folder name is required/i)).toBeTruthy();
  });

  it("rejects names containing slashes", async () => {
    const { getByLabelText, getByRole, getByText } = renderDialog(true);
    const input = getByLabelText(/folder name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "bad/name" } });
    await fireEvent.click(getByRole("button", { name: /create/i }));

    expect(onConfirm).not.toHaveBeenCalled();
    expect(getByText(/invalid characters/i)).toBeTruthy();
  });

  it("rejects names containing backslashes or colons", async () => {
    const { getByLabelText, getByRole } = renderDialog(true);
    const input = getByLabelText(/folder name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "bad\\name" } });
    await fireEvent.click(getByRole("button", { name: /create/i }));
    expect(onConfirm).not.toHaveBeenCalled();

    await fireEvent.input(input, { target: { value: "bad:name" } });
    await fireEvent.click(getByRole("button", { name: /create/i }));
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("rejects names equal to '.' or '..'", async () => {
    const { getByLabelText, getByRole } = renderDialog(true);
    const input = getByLabelText(/folder name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "." } });
    await fireEvent.click(getByRole("button", { name: /create/i }));
    expect(onConfirm).not.toHaveBeenCalled();

    await fireEvent.input(input, { target: { value: ".." } });
    await fireEvent.click(getByRole("button", { name: /create/i }));
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("calls onCancel when Cancel button is clicked", async () => {
    const { getByRole } = renderDialog(true);

    await fireEvent.click(getByRole("button", { name: /cancel/i }));

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("calls onCancel when Escape key is pressed", async () => {
    const { getByRole } = renderDialog(true);
    const dialog = getByRole("dialog");

    await fireEvent.keyDown(dialog, { key: "Escape" });

    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("confirms when Enter is pressed in the input with a valid name", async () => {
    const { getByLabelText } = renderDialog(true);
    const input = getByLabelText(/folder name/i) as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "ok-name" } });
    await fireEvent.keyDown(input, { key: "Enter" });

    expect(onConfirm).toHaveBeenCalledWith("ok-name");
  });

  it("clears the input and error when reopened via prop toggle", async () => {
    const { rerender, getByLabelText, getByRole } = renderDialog(true);
    const input1 = getByLabelText(/folder name/i) as HTMLInputElement;

    await fireEvent.input(input1, { target: { value: "old" } });
    await fireEvent.click(getByRole("button", { name: /create/i }));
    expect(input1.value).toBe("old");

    await rerender({ open: false, onConfirm, onCancel });
    await rerender({ open: true, onConfirm, onCancel });

    const input2 = getByLabelText(/folder name/i) as HTMLInputElement;
    expect(input2.value).toBe("");
  });
});
