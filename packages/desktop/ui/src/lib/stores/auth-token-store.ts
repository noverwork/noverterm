import { load } from "@tauri-apps/plugin-store";

export interface AuthSessionTokens {
  access_token: string;
  refresh_token: string;
  access_token_expires_at: string;
  email: string;
}

const TOKEN_STORE_FILE = "auth-tokens.json";
const TOKEN_KEY = "auth";

let tokenStorePromise: ReturnType<typeof load> | null = null;

function getTokenStore() {
  tokenStorePromise ??= load(TOKEN_STORE_FILE, { autoSave: false, defaults: {} });
  return tokenStorePromise;
}

export async function loadStoredAuthTokens(): Promise<AuthSessionTokens | null> {
  const store = await getTokenStore();
  const tokens = await store.get<AuthSessionTokens>(TOKEN_KEY);
  return tokens ?? null;
}

export async function saveStoredAuthTokens(tokens: AuthSessionTokens): Promise<void> {
  const store = await getTokenStore();
  await store.set(TOKEN_KEY, tokens);
  await store.save();
}

export async function clearStoredAuthTokens(): Promise<void> {
  const store = await getTokenStore();
  await store.delete(TOKEN_KEY);
  await store.save();
}
