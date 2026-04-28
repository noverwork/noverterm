import { z } from "zod";

export const connectionSchema = z
  .object({
    name: z.string().min(1, "Connection name is required"),
    host: z.string().min(1, "Host is required"),
    port: z.coerce
      .number()
      .min(1, "Port must be between 1 and 65535")
      .max(65535),
    username: z.string().min(1, "Username is required"),
    password: z.string(),
    privateKey: z.string(),
    passphrase: z.string(),
    useSshKey: z.boolean().default(false),
    keyMode: z.enum(["saved", "new"]).default("saved"),
    selectedKeyId: z.string().nullable(),
    existingPassword: z.boolean().default(false),
  })
  .superRefine((data, ctx) => {
    if (
      data.useSshKey &&
      data.keyMode === "new" &&
      !data.privateKey.trim().length
    ) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Paste a private key or select a saved key",
        path: ["privateKey"],
      });
    }
  });

export type ConnectionForm = z.infer<typeof connectionSchema>;
