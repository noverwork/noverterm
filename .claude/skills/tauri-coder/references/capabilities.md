# Tauri Capabilities Reference

## What Are Capabilities?

Capabilities define what permissions a window has. They replace the old "allowlist" approach from Tauri v1.

## Structure

```json
{
  "identifier": "unique-capability-id",
  "description": "What this capability enables",
  "local": true,
  "windows": ["main", "settings"],
  "permissions": [
    "core:default",
    "opener:default",
    "store:default",
    {
      "identifier": "http:default",
      "allow": [{ "url": "https://*" }]
    }
  ]
}
```

## Key Fields

- **`identifier`**: Unique string for this capability (used for debugging)
- **`windows`**: Array of window labels this capability applies to
- **`permissions`**: Array of permission strings or scoped permission objects
- **`local`**: `true` means this capability applies to local (non-remote) URLs

## Window Scoping

```json
{
  "identifier": "dialog-capability",
  "windows": ["main"],
  "permissions": ["dialog:allow-open"]
}
```

- **[Official]** Permissions are ONLY available to windows listed in `windows`
- **[Official]** A window can have multiple capabilities applied
- **[Common]** Separate sensitive permissions (fs, dialog) into their own capability files

## Permission Format

```
"plugin:default"          // All default permissions of a plugin
"plugin:allow-action"     // Specific allow permission
"plugin:deny-action"      // Explicit deny (overrides allow)
```

### HTTP Scoped Permissions

```json
{
  "identifier": "http:default",
  "allow": [
    { "url": "https://api.example.com/*" },
    { "url": "http://localhost:3000/*" }
  ]
}
```

### Filesystem Scoped Permissions

```json
{
  "identifier": "fs-read-appdata",
  "permissions": [
    "fs:allow-appdata-read",
    "fs:allow-appdata-write"
  ]
}
```

## Project's Current Capabilities

Located at `packages/desktop/capabilities/default.json`:

```json
{
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "store:default",
    {
      "identifier": "http:default",
      "allow": [{ "url": "http://*" }, { "url": "https://*" }]
    }
  ]
}
```

- **[Project]** Window label is `"main"` — all capabilities must reference this
- **[Common]** Add new capability files (not edit `default.json`) when adding new permission domains
