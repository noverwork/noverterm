import { beforeEach, describe, expect, it } from "vitest";

import { decryptSecret, encryptSecret, isVaultCiphertext, unlockVaultWithPassword } from "$lib/crypto/vault.js";

describe("vault crypto", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("encrypts and decrypts secrets using the login password-derived vault key", async () => {
    await unlockVaultWithPassword("Alice@Example.com", "wonderland");

    const encrypted = await encryptSecret("ssh-secret");

    expect(encrypted).not.toBe("ssh-secret");
    expect(isVaultCiphertext(encrypted)).toBe(true);
    await expect(decryptSecret(encrypted)).resolves.toBe("ssh-secret");
  });

  it("passes through existing plaintext for migration compatibility", async () => {
    await expect(decryptSecret("legacy-secret")).resolves.toBe("legacy-secret");
  });
});
