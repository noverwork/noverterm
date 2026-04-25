import { z } from "zod";

export const settingsSchema = z.object({
  fontSize: z.coerce.number().min(8, "Font size must stay between 8 and 32").max(32),
  fontFamily: z.string().min(1),
  cursorStyle: z.enum(["block", "underline", "bar"]),
  cursorBlink: z.boolean(),
  scrollback: z.coerce.number().min(100, "Scrollback must stay between 100 and 50000").max(50000),
});

export type SettingsForm = z.infer<typeof settingsSchema>;
