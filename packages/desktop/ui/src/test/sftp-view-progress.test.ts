import { afterEach, describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";
import { tick } from "svelte";

import type { TransferProgress } from "$lib/types/sftp.js";

type EventCallback<T = unknown> = (event: { payload: T }) => void;
const eventListeners = new Map<string, EventCallback>();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === "sftp_open") return "sftp-1";
    if (cmd === "sftp_connect_direct") return { status: "connected", session_id: "direct-sftp-1" };
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

afterEach(() => {
  sftpStore.cleanup();
  eventListeners.clear();
});

describe("sftp-view progress bar", () => {
  it("shows the status bar when a progress event arrives", async () => {
    await sftpStore.right.openSftp("ssh-1");
    expect(sftpStore.right.isConnected).toBe(true);

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

  it.each(["ssh", "direct"])("keeps %s machine identity across view remounts", async (source) => {
    const connection = {
      name: "Production",
      host: "prod.example.com",
      port: 2222,
      username: "deploy",
      password: "not-display-metadata",
    };
    if (source === "ssh") {
      await sftpStore.right.openSftp("ssh-production", connection);
    } else {
      await sftpStore.right.connectDirect(connection);
    }
    connection.name = "Different machine";
    connection.host = "other.example.com";
    sftpStore.right.path = "/var/www";

    const props = {
      connections: [],
      onConnect: async () => {},
      onDisconnect: async () => {},
    };
    const firstView = render(SftpView, props);
    const firstHeader = firstView.getByTestId("right-connection-identity");
    expect(firstHeader.textContent).toContain("Production");
    expect(firstHeader.textContent).toContain("deploy@prod.example.com:2222");
    expect(firstHeader.textContent).not.toContain("Different machine");
    expect(sftpStore.right.connection).not.toHaveProperty("password");
    firstView.unmount();

    const remountedView = render(SftpView, props);
    expect(remountedView.getByTestId("right-connection-identity").textContent)
      .toContain("deploy@prod.example.com:2222");
    expect(remountedView.getByLabelText("Right path")).toHaveProperty("value", "/var/www");
    await sftpStore.right.disconnect();
    await tick();
    expect(remountedView.queryByTestId("right-connection-identity")).toBeNull();
    expect(remountedView.getByText("Select a connection")).toBeTruthy();
  });
});
