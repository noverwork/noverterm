import { z } from "zod";

export const quickConnectSchema = z
  .object({
    host: z.string().min(1, "Host is required"),
    port: z.coerce.number().min(1, "Port must be 1-65535").max(65535),
    username: z.string().min(1, "Username is required"),
    password: z.string(),
    privateKey: z.string(),
  })
  .superRefine((data, ctx) => {
    if (!data.password && !data.privateKey.trim()) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Password or private key is required",
        path: ["password"],
      });
    }
  });

export type QuickConnectForm = z.infer<typeof quickConnectSchema>;
