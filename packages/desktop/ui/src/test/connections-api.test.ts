import { beforeEach, describe, expect, it, vi } from "vitest";

const mockRequestWithAuth = vi.fn();
const mockRequestNoContentWithAuth = vi.fn();
const mockWithAuthorizedRetry = vi.fn();

vi.mock("$lib/api/api-client.js", () => ({
  HttpError: class HttpError extends Error {
    constructor(
      readonly status: number,
      message: string,
    ) {
      super(message);
    }
  },
  requestWithAuth: (...args: unknown[]) => mockRequestWithAuth(...args),
  requestNoContentWithAuth: (...args: unknown[]) =>
    mockRequestNoContentWithAuth(...args),
  withAuthorizedRetry: (fn: (token: string) => unknown) =>
    mockWithAuthorizedRetry(fn),
}));

import {
  deleteBackendHostGroup,
  saveBackendConnection,
} from "$lib/api/connections-api.js";
import {
  decryptSecret,
  isVaultCiphertext,
  unlockVaultWithPassword,
} from "$lib/crypto/vault.js";

describe("connections API", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    localStorage.clear();
    mockWithAuthorizedRetry.mockImplementation((fn) => fn("test-access-token"));
    await unlockVaultWithPassword("alice@example.com", "wonderland");
  });

  it("encrypts host and key secrets before saving to the cloud backend", async () => {
    mockRequestWithAuth
      .mockResolvedValueOnce({
        id: "k1",
        name: "prod key",
        kind: "inline",
        fingerprint: null,
      })
      .mockResolvedValueOnce({
        id: "h1",
        name: "prod",
        host: "prod.example.com",
        port: 22,
        username: "deploy",
        ssh_key_id: "k1",
        group_id: "g1",
        auth: null,
      });

    await saveBackendConnection({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      groupId: "g1",
      password: "server-password",
      privateKey: "private-key",
      passphrase: "key-passphrase",
      keyName: "prod key",
    });

    const keyBody = JSON.parse(
      mockRequestWithAuth.mock.calls[0][2].body as string,
    ) as {
      encrypted_private_key: string;
      encrypted_passphrase: string;
    };
    const hostBody = JSON.parse(
      mockRequestWithAuth.mock.calls[1][2].body as string,
    ) as {
      encrypted_password: string;
      group_id: string | null;
    };

    expect(isVaultCiphertext(keyBody.encrypted_private_key)).toBe(true);
    expect(isVaultCiphertext(keyBody.encrypted_passphrase)).toBe(true);
    expect(isVaultCiphertext(hostBody.encrypted_password)).toBe(true);
    expect(hostBody.group_id).toBe("g1");
    await expect(decryptSecret(keyBody.encrypted_private_key)).resolves.toBe(
      "private-key",
    );
    await expect(decryptSecret(keyBody.encrypted_passphrase)).resolves.toBe(
      "key-passphrase",
    );
    await expect(decryptSecret(hostBody.encrypted_password)).resolves.toBe(
      "server-password",
    );
  });

  it("saves host metadata without auth material", async () => {
    mockRequestWithAuth.mockResolvedValueOnce({
      id: "h1",
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      ssh_key_id: null,
      group_id: null,
      auth: null,
    });

    await saveBackendConnection({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
    });

    expect(mockRequestWithAuth).toHaveBeenCalledTimes(1);
    const hostBody = JSON.parse(
      mockRequestWithAuth.mock.calls[0][2].body as string,
    ) as {
      ssh_key_id: string | null;
      encrypted_password: string | null;
      group_id: string | null;
    };

    expect(hostBody.ssh_key_id).toBeNull();
    expect(hostBody.encrypted_password).toBeNull();
    expect(hostBody.group_id).toBeNull();
  });

  it("clears ssh key when existingKeyId is empty string", async () => {
    mockRequestWithAuth.mockResolvedValueOnce({
      id: "h1",
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      ssh_key_id: null,
      group_id: null,
      auth: null,
    });

    await saveBackendConnection({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      existingKeyId: "",
    });

    expect(mockRequestWithAuth).toHaveBeenCalledTimes(1);
    const hostBody = JSON.parse(
      mockRequestWithAuth.mock.calls[0][2].body as string,
    ) as {
      ssh_key_id: string | null;
      encrypted_password: string | null;
    };

    expect(hostBody.ssh_key_id).toBeNull();
  });

  it("deletes host groups through the authenticated no-content endpoint", async () => {
    mockRequestNoContentWithAuth.mockResolvedValueOnce(undefined);

    await deleteBackendHostGroup({ id: "g1", name: "Production" });

    expect(mockRequestNoContentWithAuth).toHaveBeenCalledWith(
      "/bootstrap/host-groups/g1",
      "test-access-token",
      { method: "DELETE" },
    );
  });
});
