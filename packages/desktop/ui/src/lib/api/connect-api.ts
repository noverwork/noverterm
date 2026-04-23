import { requestWithAuth, withAuthorizedRetry } from "./api-client.js";

interface ConnectMaterial {
  issuance_id: string;
  host_id: string;
  host: string;
  port: number;
  username: string;
  issued_for_username: string;
  issued_for_session_id: string;
  expires_at: string;
  auth:
    | { kind: "password"; password: string }
    | { kind: "public_key"; private_key: string; passphrase: string | null }
    | {
        kind: "public_key_and_password";
        private_key: string;
        passphrase: string | null;
        password: string;
      };
}

export interface IssuedConnectionMaterial {
  host: string;
  port: number;
  username: string;
  password?: string;
  privateKey?: string;
  passphrase?: string;
}

export async function issueBackendConnectionMaterial(
  connectionId: string,
): Promise<IssuedConnectionMaterial> {
  const material = await withAuthorizedRetry((accessToken) =>
    requestWithAuth<ConnectMaterial>(
      `/bootstrap/connect/${encodeURIComponent(connectionId)}/issue`,
      accessToken,
      { method: "POST" },
    ),
  );

  switch (material.auth.kind) {
    case "password":
      return {
        host: material.host,
        port: material.port,
        username: material.username,
        password: material.auth.password,
      };
    case "public_key":
      return {
        host: material.host,
        port: material.port,
        username: material.username,
        privateKey: material.auth.private_key,
        passphrase: material.auth.passphrase ?? undefined,
      };
    case "public_key_and_password":
      return {
        host: material.host,
        port: material.port,
        username: material.username,
        password: material.auth.password,
        privateKey: material.auth.private_key,
        passphrase: material.auth.passphrase ?? undefined,
      };
  }
}
