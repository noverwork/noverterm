import type { DirectSshConnectInput } from "../../bindings.js";
import { decryptSecret } from "$lib/crypto/vault.js";
import type { ConnectionConfig } from "$lib/app-data-types.js";

interface ConnectionAuthInput {
  password: string | null;
  privateKey: string | null;
  passphrase: string | null;
}

async function connectionAuthInput(
  connection: ConnectionConfig,
): Promise<ConnectionAuthInput> {
  switch (connection.auth?.kind) {
    case "password":
      return {
        password: await decryptSecret(connection.auth.password),
        privateKey: null,
        passphrase: null,
      };
    case "public_key":
      return {
        password: null,
        privateKey: await decryptSecret(connection.auth.private_key),
        passphrase: await decryptSecret(connection.auth.passphrase),
      };
    case "public_key_and_password":
      return {
        password: await decryptSecret(connection.auth.password),
        privateKey: await decryptSecret(connection.auth.private_key),
        passphrase: await decryptSecret(connection.auth.passphrase),
      };
    default:
      throw new Error("host has no connectable authentication material");
  }
}

export async function createDirectSshConnectInput(
  connection: ConnectionConfig,
): Promise<DirectSshConnectInput> {
  const auth = await connectionAuthInput(connection);

  return {
    host: connection.host,
    port: connection.port,
    username: connection.username,
    password: auth.password,
    private_key: auth.privateKey,
    passphrase: auth.passphrase,
  };
}
