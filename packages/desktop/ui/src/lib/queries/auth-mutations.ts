import { mutationOptions } from "@tanstack/svelte-query";

import type { AuthSessionStatus } from "$lib/api/auth-api.js";
import {
  loginToBackend,
  logoutFromBackend,
  registerToBackend,
  requestPasswordReset,
  resetPassword,
  restoreBackendSession,
} from "$lib/api/auth-api.js";
import { mutationKeys } from "./query-keys.js";

export interface AuthCredentials {
  email: string;
  password: string;
}

export interface ResetPasswordInput {
  token: string;
  password: string;
}

export function restoreSessionMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.restoreSession,
    mutationFn: restoreBackendSession,
  });
}

export function loginMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.login,
    mutationFn: ({ email, password }: AuthCredentials): Promise<AuthSessionStatus> =>
      loginToBackend(email, password),
  });
}

export function registerMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.register,
    mutationFn: ({ email, password }: AuthCredentials): Promise<AuthSessionStatus> =>
      registerToBackend(email, password),
  });
}

export function logoutMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.logout,
    mutationFn: logoutFromBackend,
  });
}

export function forgotPasswordMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.forgotPassword,
    mutationFn: requestPasswordReset,
  });
}

export function resetPasswordMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.resetPassword,
    mutationFn: ({ token, password }: ResetPasswordInput): Promise<void> =>
      resetPassword(token, password),
  });
}
