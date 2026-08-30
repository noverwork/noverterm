import type { DirectSshConnectInput } from "../../bindings.js";
import type { ConnectionConfig } from "$lib/app-data-types.js";

interface ConnectionAuthInput {
  password: string | null;
  privateKey: string | null;
  passphrase: string | null;
}

function connectionAuthInput(connection: ConnectionConfig): ConnectionAuthInput {
  switch (connection.auth?.kind) {
    case "password":
      return {
        password: connection.auth.password,
        privateKey: null,
        passphrase: null,
      };
    case "public_key":
      return {
        password: null,
        privateKey: connection.auth.private_key,
        passphrase: connection.auth.passphrase,
      };
    case "public_key_and_password":
      return {
        password: connection.auth.password,
        privateKey: connection.auth.private_key,
        passphrase: connection.auth.passphrase,
      };
    default:
      throw new Error("host has no connectable authentication material");
  }
}

export function createDirectSshConnectInput(
  connection: ConnectionConfig,
): DirectSshConnectInput {
  const auth = connectionAuthInput(connection);

  return {
    host: connection.host,
    port: connection.port,
    username: connection.username,
    password: auth.password,
    private_key: auth.privateKey,
    passphrase: auth.passphrase,
  };
}
