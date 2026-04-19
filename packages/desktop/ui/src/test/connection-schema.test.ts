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
      hasExistingKey: false,
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
      hasExistingKey: false,
    });

    expect(result.success).toBe(true);
  });

  it("requires either a password or an ssh key", () => {
    const result = connectionSchema.safeParse({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "",
      privateKey: "",
      passphrase: "",
      useSshKey: false,
      hasExistingKey: false,
    });

    expect(result.success).toBe(false);
    expect(result.error?.flatten().fieldErrors.password).toContain("Enter a password or use an SSH key");
  });
});
