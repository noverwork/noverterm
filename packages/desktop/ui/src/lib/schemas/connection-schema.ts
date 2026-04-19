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
    useSshKey: z.boolean().default(false),
    hasExistingKey: z.boolean().default(false),
  })
  .superRefine((data, ctx) => {
    const hasPassword = data.password.trim().length > 0;
    const hasPrivateKey = data.privateKey.trim().length > 0 || (data.useSshKey && data.hasExistingKey);

    if (!hasPassword && !hasPrivateKey) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Enter a password or use an SSH key",
        path: ["password"],
      });
    }

    if (data.useSshKey && !hasPrivateKey) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Paste a private key or keep the existing saved key",
        path: ["privateKey"],
      });
    }
  });

export type ConnectionForm = z.infer<typeof connectionSchema>;
