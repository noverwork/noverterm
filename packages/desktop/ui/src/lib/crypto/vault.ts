const VAULT_PREFIX = "nvlt:v1";
const ACTIVE_EMAIL_KEY = "noverterm:vault:active-email";
const DERIVE_ITERATIONS = 310_000;

let cachedEmail: string | null = null;
let cachedKey: CryptoKey | null = null;

function normalizedEmail(email: string): string {
  return email.trim().toLowerCase();
}

function keyStorageKey(email: string): string {
  return `noverterm:vault:key:${normalizedEmail(email)}`;
}

function textBytes(value: string): Uint8Array {
  return new TextEncoder().encode(value);
}

function bytesText(value: ArrayBuffer): string {
  return new TextDecoder().decode(value);
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

function base64ToBytes(value: string): Uint8Array {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

async function importAesKey(rawKey: Uint8Array): Promise<CryptoKey> {
  return crypto.subtle.importKey("raw", rawKey, "AES-GCM", false, ["encrypt", "decrypt"]);
}

async function exportAesKey(key: CryptoKey): Promise<Uint8Array> {
  return new Uint8Array(await crypto.subtle.exportKey("raw", key));
}

async function deriveVaultKey(email: string, password: string): Promise<CryptoKey> {
  const keyMaterial = await crypto.subtle.importKey(
    "raw",
    textBytes(password),
    "PBKDF2",
    false,
    ["deriveKey"],
  );

  return crypto.subtle.deriveKey(
    {
      name: "PBKDF2",
      hash: "SHA-256",
      salt: textBytes(`noverterm:vault:${normalizedEmail(email)}`),
      iterations: DERIVE_ITERATIONS,
    },
    keyMaterial,
    { name: "AES-GCM", length: 256 },
    true,
    ["encrypt", "decrypt"],
  );
}

export function isVaultCiphertext(value: string | null | undefined): value is string {
  return typeof value === "string" && value.startsWith(`${VAULT_PREFIX}:`);
}

export async function unlockVaultWithPassword(email: string, password: string): Promise<void> {
  const vaultKey = await deriveVaultKey(email, password);
  const rawKey = await exportAesKey(vaultKey);
  const normalized = normalizedEmail(email);

  localStorage.setItem(keyStorageKey(normalized), bytesToBase64(rawKey));
  localStorage.setItem(ACTIVE_EMAIL_KEY, normalized);
  cachedEmail = normalized;
  cachedKey = vaultKey;
}

export function setActiveVaultEmail(email: string): void {
  const normalized = normalizedEmail(email);
  localStorage.setItem(ACTIVE_EMAIL_KEY, normalized);
  if (cachedEmail !== normalized) {
    cachedEmail = null;
    cachedKey = null;
  }
}

async function loadVaultKey(): Promise<CryptoKey> {
  const email = localStorage.getItem(ACTIVE_EMAIL_KEY);
  if (!email) {
    throw new Error("vault is locked");
  }

  if (cachedEmail === email && cachedKey) {
    return cachedKey;
  }

  const encodedKey = localStorage.getItem(keyStorageKey(email));
  if (!encodedKey) {
    throw new Error("vault is locked");
  }

  const key = await importAesKey(base64ToBytes(encodedKey));
  cachedEmail = email;
  cachedKey = key;
  return key;
}

export async function encryptSecret(value: string | null): Promise<string | null> {
  if (!value) {
    return null;
  }

  if (isVaultCiphertext(value)) {
    return value;
  }

  const key = await loadVaultKey();
  const nonce = crypto.getRandomValues(new Uint8Array(12));
  const ciphertext = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv: nonce },
    key,
    textBytes(value),
  );

  return `${VAULT_PREFIX}:${bytesToBase64(nonce)}:${bytesToBase64(new Uint8Array(ciphertext))}`;
}

export async function decryptSecret(value: string | null | undefined): Promise<string | null> {
  if (!value) {
    return null;
  }

  if (!isVaultCiphertext(value)) {
    return value;
  }

  const [, , encodedNonce, encodedCiphertext] = value.split(":");
  if (!encodedNonce || !encodedCiphertext) {
    throw new Error("invalid vault ciphertext");
  }

  const key = await loadVaultKey();
  const plaintext = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: base64ToBytes(encodedNonce) },
    key,
    base64ToBytes(encodedCiphertext),
  );

  return bytesText(plaintext);
}
