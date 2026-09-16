import { describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";
import { tick } from "svelte";

import type { TransferProgress } from "$lib/types/sftp.js";

type EventCallback<T = unknown> = (event: { payload: T }) => void;
const eventListeners = new Map<string, EventCallback>();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === "sftp_open") return "sftp-1";
    if (cmd === "sftp_list_dir" || cmd === "local_list_dir") return [];
    if (cmd === "sftp_home_dir") return "/";
    return undefined;
  }),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, cb: EventCallback) => {
    eventListeners.set(name, cb);
    return Promise.resolve(() => eventListeners.delete(name));
  },
}));

import SftpView from "$lib/components/sftp-view.svelte";
import { sftpStore } from "$lib/stores/sftp.svelte.js";

describe("sftp-view progress bar", () => {
  it("shows the status bar when a progress event arrives", async () => {
    await sftpStore.openSftp("ssh-1");
    expect(sftpStore.isConnected).toBe(true);

    const { container } = render(SftpView, {
      connections: [],
      onConnect: async () => {},
      onDisconnect: async () => {},
    });
    await tick();
    expect(container.querySelector('[data-testid="transfer-status-bar"]')).toBeNull();

    const progress: TransferProgress = {
      transfer_id: "t-1",
      bytes_transferred: 1024 * 1024,
      total_bytes: 2_600_000_000,
      speed_bps: 10_000_000,
      direction: "Upload",
    };
    eventListeners.get("sftp://progress")!({ payload: progress });
    await tick();

    const bar = container.querySelector('[data-testid="transfer-status-bar"]');
    expect(bar, "status bar should render after progress event").not.toBeNull();
    expect(bar!.textContent).toContain("1 active");
  });
});
