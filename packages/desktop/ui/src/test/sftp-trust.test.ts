import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/svelte";
import { tick } from "svelte";
import { SvelteDate, SvelteMap } from "svelte/reactivity";

import type { ConnectionConfig } from "$lib/app-data-types.js";
import type { Session } from "$lib/stores/session.svelte.js";
import type { HostTrustMismatch, HostTrustPrompt, SshConnectResponse } from "../bindings.js";

const mocks = vi.hoisted(() => ({
  app: vi.fn(),
  connect: vi.fn<() => Promise<SshConnectResponse>>(),
  open: vi.fn<() => Promise<string>>(),
  confirm: vi.fn<() => Promise<null>>(),
}));

vi.mock("$lib/stores/app-shell.svelte.js", () => ({ getAppShellContext: mocks.app }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

import { invoke } from "@tauri-apps/api/core";
import SftpPage from "../routes/sftp/+page.svelte";
import { sftpStore } from "$lib/stores/sftp.svelte.js";

const connection: ConnectionConfig = {
  id: "saved-production",
  name: "Production",
  host: "prod.example.com",
  port: 2222,
  username: "deploy",
  groupId: null,
  sshKeyId: null,
  hasPassword: true,
  auth: { kind: "password", password: "never-display-this-secret" },
};
const prompt: HostTrustPrompt = {
  host: connection.host,
  port: connection.port,
  algorithm: "ssh-ed25519",
  fingerprint: "SHA256:presented-key",
};
const mismatch: HostTrustMismatch = {
  host: prompt.host,
  port: prompt.port,
  expected_algorithm: "ssh-ed25519",
  expected_fingerprint: "SHA256:old-key",
  presented_algorithm: prompt.algorithm,
  presented_fingerprint: prompt.fingerprint,
};
const connected: SshConnectResponse = { status: "connected", session_id: "sftp-production" };
const terminal: Session = {
  id: "terminal-production", type: "ssh", status: "connected", name: "Production terminal",
  host: connection.host, port: connection.port, username: connection.username,
  createdAt: new SvelteDate(0),
};
let app: { connections: ConnectionConfig[]; activeSessions: Session[]; activeSession: Session | null };

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.connect.mockReset();
  mocks.open.mockReset().mockResolvedValue("attached-sftp");
  mocks.confirm.mockReset().mockResolvedValue(null);
  const activeSessions = new SvelteMap<string, Session>();
  app = {
    connections: [connection],
    activeSessions: [],
    get activeSession() { return activeSessions.get("active") ?? null; },
    set activeSession(session: Session | null) {
      if (session) activeSessions.set("active", session);
      else activeSessions.delete("active");
    },
  };
  mocks.app.mockReturnValue(app);
  vi.mocked(invoke).mockImplementation(async (command: string) => {
    if (command === "sftp_connect_direct") return await mocks.connect();
    if (command === "sftp_open") return await mocks.open();
    if (command === "ssh_confirm_host_trust") return await mocks.confirm();
    if (command === "sftp_list_dir" || command === "local_list_dir") return [];
    if (command === "sftp_home_dir") return "/home/deploy";
    return null;
  });
});

afterEach(() => {
  cleanup();
  sftpStore.cleanup();
});

async function selectConnection() {
  const view = render(SftpPage);
  await fireEvent.click(view.getByRole("button", { name: /Production/ }));
  return view;
}

describe("SFTP host trust", () => {
  it("waits for explicit trust, survives remount, and retries the selected saved connection", async () => {
    mocks.connect.mockResolvedValueOnce({ status: "trust_required", prompt }).mockResolvedValueOnce(connected);
    const firstView = await selectConnection();
    await firstView.findByText("Verify SSH host identity");
    expect(mocks.confirm).not.toHaveBeenCalled();
    expect(sftpStore.right.isConnected).toBe(false);
    expect(sftpStore.right.connection).not.toHaveProperty("password");
    expect(firstView.container.textContent).not.toContain("never-display-this-secret");
    firstView.unmount();

    app.activeSession = {
      id: "other-terminal", type: "ssh", status: "connected", connectionId: "other-saved",
      name: "Other terminal", host: "other.example.com", port: 22, username: "other",
      createdAt: new SvelteDate(0),
    };
    const view = render(SftpPage);
    await fireEvent.click(view.getByRole("button", { name: "Trust host and retry" }));
    await waitFor(() => expect(sftpStore.right.sftpSessionId).toBe("sftp-production"));
    expect(invoke).toHaveBeenCalledWith("ssh_confirm_host_trust", { confirmation: prompt });
    expect(mocks.confirm).toHaveBeenCalledTimes(1);
    expect(mocks.connect).toHaveBeenCalledTimes(2);
    await waitFor(() => expect(invoke).toHaveBeenCalledWith(
      "sftp_list_dir", { sessionId: "sftp-production", path: "/home/deploy" },
    ));
    expect(vi.mocked(invoke).mock.calls.filter(([command]) => command === "sftp_connect_direct").map(([, args]) => args))
      .toEqual([1, 2].map(() => ({
        host: connection.host, port: connection.port, username: connection.username,
        password: "never-display-this-secret", privateKey: null, passphrase: null,
      })));
    expect(invoke).not.toHaveBeenCalledWith("sftp_open", expect.anything());
  });

  it("keeps mismatches blocked on retry until explicit key replacement", async () => {
    mocks.connect.mockResolvedValue({ status: "trust_mismatch", mismatch });
    const view = await selectConnection();
    await view.findByText("Saved fingerprint does not match.");
    expect(view.getByText(mismatch.expected_fingerprint)).toBeTruthy();
    expect(view.getByText(mismatch.presented_fingerprint)).toBeTruthy();
    expect(mocks.confirm).not.toHaveBeenCalled();
    await fireEvent.click(view.getByRole("button", { name: "Retry session" }));
    await waitFor(() => expect(mocks.connect).toHaveBeenCalledTimes(2));
    expect(mocks.confirm).not.toHaveBeenCalled();
    expect(sftpStore.right.isConnected).toBe(false);

    mocks.connect.mockResolvedValueOnce(connected);
    await fireEvent.click(view.getByRole("button", { name: "Delete & trust new key" }));
    await waitFor(() => expect(sftpStore.right.sftpSessionId).toBe("sftp-production"));
    expect(invoke).toHaveBeenCalledWith("ssh_confirm_host_trust", { confirmation: prompt });
  });

  it("shows trust-save rejection without reconnecting or automatically retrying", async () => {
    mocks.connect.mockResolvedValue({ status: "trust_required", prompt });
    mocks.confirm.mockRejectedValue("Known Hosts database is read-only");
    const view = await selectConnection();
    await fireEvent.click(await view.findByRole("button", { name: "Trust host and retry" }));
    await view.findByText("Known Hosts database is read-only");
    expect(mocks.connect).toHaveBeenCalledTimes(1);
    expect(mocks.confirm).toHaveBeenCalledTimes(1);
    expect(sftpStore.right.isConnected).toBe(false);
    expect(view.getByRole("button", { name: "Trust host and retry" })).toHaveProperty("disabled", false);
    await fireEvent.click(view.getByRole("button", { name: "Cancel" }));
    expect(sftpStore.right.trustPrompt).toBeNull();
    expect(view.queryByText("Known Hosts database is read-only")).toBeNull();
  });

  it("preserves failed identity across remount and offers retry and cancel", async () => {
    mocks.connect.mockRejectedValueOnce("Authentication failed").mockResolvedValueOnce(connected);
    const firstView = await selectConnection();
    await firstView.findByText("Authentication failed");
    expect(sftpStore.right.connection?.name).toBe("Production");
    expect(sftpStore.right.connectionId).toBe(connection.id);
    firstView.unmount();
    const view = render(SftpPage);
    expect(view.getByRole("button", { name: "Cancel" })).toBeTruthy();
    await fireEvent.click(view.getByRole("button", { name: "Retry session" }));
    await waitFor(() => expect(sftpStore.right.sftpSessionId).toBe("sftp-production"));
    expect(mocks.confirm).not.toHaveBeenCalled();
  });

  it("does not reconnect after cancellation during trust persistence", async () => {
    const saving = deferred<null>();
    mocks.connect.mockResolvedValue({ status: "trust_required", prompt });
    mocks.confirm.mockReturnValue(saving.promise);
    const view = await selectConnection();
    await fireEvent.click(await view.findByRole("button", { name: "Trust host and retry" }));
    await waitFor(() => expect(mocks.confirm).toHaveBeenCalledOnce());
    await fireEvent.click(view.getByRole("button", { name: "Cancel" }));
    saving.resolve(null);
    await saving.promise;
    await tick();
    expect(mocks.connect).toHaveBeenCalledTimes(1);
    expect(sftpStore.right.connection).toBeNull();
    expect(sftpStore.right.connectionId).toBeNull();
    expect(sftpStore.right.trustPrompt).toBeNull();
    expect(sftpStore.right.isConnected).toBe(false);
    expect(view.getByRole("button", { name: /Production/ })).toBeTruthy();
  });

  it.each<SshConnectResponse>([
    { status: "trust_required", prompt },
    { status: "trust_mismatch", mismatch },
    connected,
  ])("discards stale $status replies after cancelling a retry", async (response) => {
    const retrying = deferred<SshConnectResponse>();
    mocks.connect.mockRejectedValueOnce("Connection refused").mockReturnValueOnce(retrying.promise);
    const view = await selectConnection();
    await fireEvent.click(await view.findByRole("button", { name: "Retry session" }));
    await view.findByText("Connecting to Production");
    await fireEvent.click(view.getByRole("button", { name: "Cancel" }));
    retrying.resolve(response);
    await retrying.promise;
    await tick();
    await tick();
    expect(sftpStore.right.connection).toBeNull();
    expect(sftpStore.right.trustPrompt).toBeNull();
    expect(sftpStore.right.trustMismatch).toBeNull();
    expect(sftpStore.right.connectionError).toBeNull();
    expect(sftpStore.right.isConnected).toBe(false);
    expect(view.queryByText("Verify SSH host identity")).toBeNull();
    if (response.status === "connected") {
      expect(invoke).toHaveBeenCalledWith("sftp_close", { sessionId: response.session_id });
    }
    view.unmount();
    const remounted = render(SftpPage);
    await tick();
    expect(remounted.getByRole("button", { name: /Production/ })).toBeTruthy();
    expect(invoke).not.toHaveBeenCalledWith("sftp_open", expect.anything());
  });

  it("shows attached-session failures and retries the same SSH session after switching terminals", async () => {
    mocks.open.mockRejectedValueOnce("SFTP subsystem unavailable").mockResolvedValueOnce("attached-retry");
    app.activeSession = terminal;
    const view = render(SftpPage);
    await view.findByText("SFTP subsystem unavailable");
    expect(sftpStore.right.connection?.name).toBe(terminal.name);
    expect(sftpStore.right.sshSessionId).toBe(terminal.id);
    expect(view.getByRole("button", { name: "Cancel" })).toBeTruthy();
    app.activeSession = { ...terminal, id: "different-terminal" };
    await tick();
    expect(mocks.open).toHaveBeenCalledTimes(1);
    await fireEvent.click(view.getByRole("button", { name: "Retry session" }));
    await waitFor(() => expect(sftpStore.right.sftpSessionId).toBe("attached-retry"));
    await tick();
    expect(vi.mocked(invoke).mock.calls.filter(([command]) => command === "sftp_open").map(([, args]) => args))
      .toEqual([{ sessionId: terminal.id }, { sessionId: terminal.id }]);
    expect(mocks.connect).not.toHaveBeenCalled();
  });

  it("cancels an attached failure without reopening it, but allows a newly active SSH session", async () => {
    mocks.open.mockRejectedValueOnce("SFTP subsystem unavailable");
    app.activeSession = terminal;
    const view = render(SftpPage);
    await view.findByText("SFTP subsystem unavailable");
    await fireEvent.click(view.getByRole("button", { name: "Cancel" }));
    expect(sftpStore.right.sshSessionId).toBeNull();
    expect(sftpStore.right.connection).toBeNull();
    view.unmount();
    const remounted = render(SftpPage);
    await tick();
    expect(remounted.getByRole("button", { name: /Production/ })).toBeTruthy();
    expect(mocks.open).toHaveBeenCalledTimes(1);
    app.activeSession = { ...terminal, id: "new-terminal" };
    await waitFor(() => expect(sftpStore.right.sftpSessionId).toBe("attached-sftp"));
    expect(invoke).toHaveBeenCalledWith("sftp_open", { sessionId: "new-terminal" });
  });

  it("allows a newly active SSH session after cancelling direct host trust", async () => {
    app.activeSession = terminal;
    sftpStore.attemptedActiveSshSessionId = terminal.id;
    mocks.connect.mockResolvedValue({ status: "trust_required", prompt });
    const view = await selectConnection();
    await view.findByText("Verify SSH host identity");
    await fireEvent.click(await view.findByRole("button", { name: "Cancel" }));
    await tick();
    expect(mocks.open).not.toHaveBeenCalled();
    expect(mocks.confirm).not.toHaveBeenCalled();
    app.activeSession = { ...terminal, id: "new-terminal" };
    await waitFor(() => expect(sftpStore.right.sftpSessionId).toBe("attached-sftp"));
    expect(invoke).toHaveBeenCalledWith("sftp_open", { sessionId: "new-terminal" });
  });

  it("lets the left pane switch from this machine to an open SSH session", async () => {
    app.activeSessions = [terminal];
    const view = render(SftpPage);
    const left = within(view.container.querySelector<HTMLElement>('[data-side="left"]')!);
    await fireEvent.click(left.getByRole("button", { name: "Switch machine" }));
    await fireEvent.click(left.getByRole("button", { name: /Production terminal/ }));
    await waitFor(() => expect(sftpStore.left.sftpSessionId).toBe("attached-sftp"));
    expect(invoke).toHaveBeenCalledWith("sftp_open", { sessionId: terminal.id });
    expect(sftpStore.right.sftpSessionId).toBeNull();
  });

  it("does not offer the machine already open on the other side", async () => {
    app.activeSessions = [terminal];
    mocks.connect.mockResolvedValueOnce(connected);
    const view = render(SftpPage);
    const pane = (side: string) =>
      within(view.container.querySelector<HTMLElement>(`[data-side="${side}"]`)!);

    expect(pane("right").getByRole("button", { name: /^Local/ })).toHaveProperty("disabled", true);
    await fireEvent.click(pane("right").getByRole("button", { name: /^Production deploy/ }));
    await waitFor(() => expect(sftpStore.right.sftpSessionId).toBe("sftp-production"));

    await fireEvent.click(pane("left").getByRole("button", { name: "Switch machine" }));
    expect(pane("left").getByRole("button", { name: /^Local/ })).toHaveProperty("disabled", false);
    expect(pane("left").getByRole("button", { name: /Production terminal/ })).toHaveProperty("disabled", true);
    expect(pane("left").getByRole("button", { name: /^Production deploy/ })).toHaveProperty("disabled", true);
  });
});
