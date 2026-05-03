import { beforeEach, describe, expect, it, vi } from "vitest";

const mockRequestWithAuth = vi.fn();
const mockRequestNoContentWithAuth = vi.fn();
const mockWithAuthorizedRetry = vi.fn();

vi.mock("$lib/api/api-client.js", () => ({
  requestWithAuth: (...args: unknown[]) => mockRequestWithAuth(...args),
  requestNoContentWithAuth: (...args: unknown[]) => mockRequestNoContentWithAuth(...args),
  withAuthorizedRetry: (fn: (token: string) => unknown) => mockWithAuthorizedRetry(fn),
}));

import { createSshKey, updateSshKey, deleteSshKey, revealSshKeySecret } from "$lib/api/keys-api.js";
import {
  decryptSecret,
  encryptSecret,
  isVaultCiphertext,
  unlockVaultWithPassword,
} from "$lib/crypto/vault.js";
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
  beforeEach(async () => {
    vi.clearAllMocks();
    localStorage.clear();
    mockWithAuthorizedRetry.mockImplementation((fn) => fn("test-access-token"));
    await unlockVaultWithPassword("alice@example.com", "wonderland");
  });

  describe("createSshKey", () => {
    it("posts to /bootstrap/keys with correct payload", async () => {
      mockRequestWithAuth.mockResolvedValue(sampleKey);

      const result = await createSshKey(sampleCreateRequest);

      expect(mockWithAuthorizedRetry).toHaveBeenCalledOnce();
      expect(mockRequestWithAuth).toHaveBeenCalledWith("/bootstrap/keys", "test-access-token", {
        method: "POST",
        body: expect.any(String),
      });
      const body = JSON.parse(mockRequestWithAuth.mock.calls[0][2].body as string) as KeyCreateRequest;
      expect(isVaultCiphertext(body.encrypted_private_key)).toBe(true);
      await expect(decryptSecret(body.encrypted_private_key)).resolves.toBe(
        sampleCreateRequest.encrypted_private_key,
      );
      expect(result).toEqual(sampleKey);
    });
  });

  describe("updateSshKey", () => {
    it("puts to /bootstrap/keys/:id with correct payload", async () => {
      mockRequestWithAuth.mockResolvedValue({ ...sampleKey, name: "updated-key" });

      const result = await updateSshKey("k1", sampleUpdateRequest);

      expect(mockWithAuthorizedRetry).toHaveBeenCalledOnce();
      expect(mockRequestWithAuth).toHaveBeenCalledWith("/bootstrap/keys/k1", "test-access-token", {
        method: "PUT",
        body: expect.any(String),
      });
      const body = JSON.parse(mockRequestWithAuth.mock.calls[0][2].body as string) as KeyUpdateRequest;
      expect(isVaultCiphertext(body.encrypted_private_key)).toBe(true);
      expect(isVaultCiphertext(body.encrypted_passphrase)).toBe(true);
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

    it("supports name-only updates without resending private key material", async () => {
      const nameOnlyRequest: KeyUpdateRequest = {
        name: "renamed-key",
        kind: "inline",
      };
      mockRequestWithAuth.mockResolvedValue({ ...sampleKey, name: "renamed-key" });

      const result = await updateSshKey("k1", nameOnlyRequest);

      expect(mockRequestWithAuth).toHaveBeenCalledWith(
        "/bootstrap/keys/k1",
        "test-access-token",
        {
          method: "PUT",
          body: JSON.stringify(nameOnlyRequest),
        },
      );
      expect(result.name).toBe("renamed-key");
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

  describe("revealSshKeySecret", () => {
    it("fetches and decrypts the saved key secret", async () => {
      const encryptedPrivateKey = await encryptSecret("raw-private-key");
      const encryptedPassphrase = await encryptSecret("raw-passphrase");
      mockRequestWithAuth.mockResolvedValue({
        private_key: encryptedPrivateKey,
        passphrase: encryptedPassphrase,
      });

      const result = await revealSshKeySecret("k1");

      expect(mockRequestWithAuth).toHaveBeenCalledWith(
        "/bootstrap/keys/k1/secret",
        "test-access-token",
      );
      expect(result).toEqual({
        private_key: "raw-private-key",
        passphrase: "raw-passphrase",
      });
    });

    it("URL-encodes the key ID", async () => {
      mockRequestWithAuth.mockResolvedValue({
        private_key: "raw-private-key",
        passphrase: null,
      });

      await revealSshKeySecret("k/1+special");

      expect(mockRequestWithAuth).toHaveBeenCalledWith(
        "/bootstrap/keys/k%2F1%2Bspecial/secret",
        "test-access-token",
      );
    });
  });
});
