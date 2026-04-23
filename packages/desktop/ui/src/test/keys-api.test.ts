import { beforeEach, describe, expect, it, vi } from "vitest";

const mockRequestWithAuth = vi.fn();
const mockRequestNoContentWithAuth = vi.fn();
const mockWithAuthorizedRetry = vi.fn();

vi.mock("$lib/api/api-client.js", () => ({
  requestWithAuth: (...args: unknown[]) => mockRequestWithAuth(...args),
  requestNoContentWithAuth: (...args: unknown[]) => mockRequestNoContentWithAuth(...args),
  withAuthorizedRetry: (fn: (token: string) => unknown) => mockWithAuthorizedRetry(fn),
}));

import { createSshKey, updateSshKey, deleteSshKey } from "$lib/api/keys-api.js";
import type { KeyCreateRequest, KeyUpdateRequest, SshKeyRecord } from "$lib/api/types.js";

const sampleKey: SshKeyRecord = {
  id: "k1",
  name: "deploy-key",
  kind: "inline",
  fingerprint: "SHA256:abc123",
};

const sampleCreateRequest: KeyCreateRequest = {
  name: "deploy-key",
  kind: "inline",
  encrypted_private_key: "-----BEGIN OPENSSH PRIVATE KEY-----",
  encrypted_passphrase: null,
};

const sampleUpdateRequest: KeyUpdateRequest = {
  name: "updated-key",
  kind: "inline",
  encrypted_private_key: "-----BEGIN OPENSSH PRIVATE KEY-----",
  encrypted_passphrase: "new-passphrase",
};

describe("keys API", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockWithAuthorizedRetry.mockImplementation((fn) => fn("test-access-token"));
  });

  describe("createSshKey", () => {
    it("posts to /bootstrap/keys with correct payload", async () => {
      mockRequestWithAuth.mockResolvedValue(sampleKey);

      const result = await createSshKey(sampleCreateRequest);

      expect(mockWithAuthorizedRetry).toHaveBeenCalledOnce();
      expect(mockRequestWithAuth).toHaveBeenCalledWith(
        "/bootstrap/keys",
        "test-access-token",
        {
          method: "POST",
          body: JSON.stringify(sampleCreateRequest),
        },
      );
      expect(result).toEqual(sampleKey);
    });
  });

  describe("updateSshKey", () => {
    it("puts to /bootstrap/keys/:id with correct payload", async () => {
      mockRequestWithAuth.mockResolvedValue({ ...sampleKey, name: "updated-key" });

      const result = await updateSshKey("k1", sampleUpdateRequest);

      expect(mockWithAuthorizedRetry).toHaveBeenCalledOnce();
      expect(mockRequestWithAuth).toHaveBeenCalledWith(
        "/bootstrap/keys/k1",
        "test-access-token",
        {
          method: "PUT",
          body: JSON.stringify(sampleUpdateRequest),
        },
      );
      expect(result.name).toBe("updated-key");
    });

    it("URL-encodes the key ID", async () => {
      mockRequestWithAuth.mockResolvedValue(sampleKey);

      await updateSshKey("k/1+special", sampleUpdateRequest);

      expect(mockRequestWithAuth).toHaveBeenCalledWith(
        "/bootstrap/keys/k%2F1%2Bspecial",
        "test-access-token",
        expect.any(Object),
      );
    });
  });

  describe("deleteSshKey", () => {
    it("deletes from /bootstrap/keys/:id", async () => {
      mockRequestNoContentWithAuth.mockResolvedValue(undefined);

      await deleteSshKey("k1");

      expect(mockWithAuthorizedRetry).toHaveBeenCalledOnce();
      expect(mockRequestNoContentWithAuth).toHaveBeenCalledWith(
        "/bootstrap/keys/k1",
        "test-access-token",
        { method: "DELETE" },
      );
    });

    it("URL-encodes the key ID", async () => {
      mockRequestNoContentWithAuth.mockResolvedValue(undefined);

      await deleteSshKey("k/1+special");

      expect(mockRequestNoContentWithAuth).toHaveBeenCalledWith(
        "/bootstrap/keys/k%2F1%2Bspecial",
        "test-access-token",
        { method: "DELETE" },
      );
    });
  });
});
