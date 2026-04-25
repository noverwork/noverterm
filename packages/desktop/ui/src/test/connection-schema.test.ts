import { describe, expect, it } from "vitest";

import { connectionSchema } from "$lib/schemas/connection-schema.js";

describe("connection schema", () => {
  it("accepts password-only connections without selecting an auth mode", () => {
    const result = connectionSchema.safeParse({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "secret",
      privateKey: "",
      passphrase: "",
      useSshKey: false,
      keyMode: "saved",
      selectedKeyId: null,
      existingPassword: false,
    });

    expect(result.success).toBe(true);
  });

  it("accepts key-only connections when ssh key is enabled", () => {
    const result = connectionSchema.safeParse({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "",
      privateKey: "-----BEGIN OPENSSH PRIVATE KEY-----",
      passphrase: "",
      useSshKey: true,
      keyMode: "new",
      selectedKeyId: null,
      existingPassword: false,
    });

    expect(result.success).toBe(true);
  });

  it("accepts connections using a saved key", () => {
    const result = connectionSchema.safeParse({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "",
      privateKey: "",
      passphrase: "",
      useSshKey: true,
      keyMode: "saved",
      selectedKeyId: "key-123",
      existingPassword: false,
    });

    expect(result.success).toBe(true);
  });

  it("accepts connections without auth material", () => {
    const result = connectionSchema.safeParse({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "",
      privateKey: "",
      passphrase: "",
      useSshKey: false,
      keyMode: "saved",
      selectedKeyId: null,
      existingPassword: false,
    });

    expect(result.success).toBe(true);
  });

  it("accepts existing password when editing without changing auth", () => {
    const result = connectionSchema.safeParse({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "",
      privateKey: "",
      passphrase: "",
      useSshKey: false,
      keyMode: "saved",
      selectedKeyId: null,
      existingPassword: true,
    });

    expect(result.success).toBe(true);
  });

  it("requires key material when keyMode is new and no key is pasted", () => {
    const result = connectionSchema.safeParse({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "",
      privateKey: "",
      passphrase: "",
      useSshKey: true,
      keyMode: "new",
      selectedKeyId: null,
      existingPassword: false,
    });

    expect(result.success).toBe(false);
    expect(result.error?.flatten().fieldErrors.privateKey).toContain("Paste a private key or select a saved key");
  });
});
