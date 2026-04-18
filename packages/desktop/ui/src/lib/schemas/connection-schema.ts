import { z } from "zod";

export const connectionSchema = z
  .object({
    name: z.string().min(1, "Connection name is required"),
    host: z.string().min(1, "Host is required"),
    port: z.coerce.number().min(1, "Port must be between 1 and 65535").max(65535),
    username: z.string().min(1, "Username is required"),
    password: z.string(),
    privateKey: z.string(),
    passphrase: z.string(),
    authMode: z.enum(["password", "publickey", "publickey_password"]),
    hasExistingKey: z.boolean().default(false),
  })
  .superRefine((data, ctx) => {
    const hasPassword = data.password.trim().length > 0;
    const hasPrivateKey = data.privateKey.trim().length > 0 || data.hasExistingKey;

    if (data.authMode === "password" && !hasPassword) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: data.hasExistingKey
          ? "Re-enter the password to keep password authentication"
          : "Password is required",
        path: ["password"],
      });
    }

    if (data.authMode === "publickey" && !hasPrivateKey) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Paste a private key or keep the existing saved key",
        path: ["privateKey"],
      });
    }

    if (data.authMode === "publickey_password") {
      if (!hasPrivateKey) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: "A private key is required for this auth mode",
          path: ["privateKey"],
        });
      }
      if (!hasPassword) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: data.hasExistingKey
            ? "Re-enter the password to keep hybrid authentication"
            : "Password is required for this auth mode",
          path: ["password"],
        });
      }
    }
  });

export type ConnectionForm = z.infer<typeof connectionSchema>;
