import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

import ConnectionForm from "$lib/components/connection-form.svelte";
import type { ConnectionConfig } from "$lib/app-data-types.js";

function connection(id: string, jumpHostId: string | null = null): ConnectionConfig {
  return {
    id,
    name: id,
    groupId: null,
    host: `${id}.example.test`,
    port: 22,
    username: "deploy",
    sshKeyId: null,
    hasPassword: true,
    auth: { kind: "password", password: "pw" },
    jumpHostId,
    jumps: [],
  };
}

describe("connection form", () => {
  it("shows the missing username instead of silently skipping save", async () => {
    const onSave = vi.fn();
    const view = render(ConnectionForm, {
      connection: { ...connection("target"), username: "" },
      hosts: [connection("bastion")],
      onSave,
      onCancel: vi.fn(),
    });

    await fireEvent.submit(view.container.querySelector("form")!);

    expect(await view.findByText("Username is required")).toBeTruthy();
    expect(onSave).not.toHaveBeenCalled();
  });

  it("submits the selected jump host", async () => {
    const onSave = vi.fn();
    const target = connection("target");
    const view = render(ConnectionForm, {
      connection: target,
      hosts: [connection("bastion"), target],
      onSave,
      onCancel: vi.fn(),
    });

    const select = view.getByLabelText("Jump host") as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: "bastion" } });
    await fireEvent.submit(view.container.querySelector("form")!);

    await waitFor(() => expect(onSave).toHaveBeenCalled());
    expect(onSave.mock.calls[0][0]).toMatchObject({ id: "target", jumpHostId: "bastion" });
  });
});
