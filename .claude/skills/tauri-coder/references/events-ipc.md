# Events & IPC

## Commands vs Events vs Channels

| Mechanism | Use for | Return value | Type safety |
| --- | --- | --- | --- |
| Command | Request/response work from UI to Rust | Yes | Strong with `tauri-specta` |
| Event | Fire-and-forget notifications | No | JSON payloads |
| Channel | Streaming/progress/high-volume data | Event stream | Payload type is app-defined |

Prefer commands for CRUD-like operations and typed data requests. Use events for notifications and channels for progress or streaming data.

## Rust Emit

Use Tauri `Emitter` when Rust needs to notify the frontend.

```rust
use tauri::{AppHandle, Emitter};

fn notify(app: &AppHandle, message: &str) -> Result<(), tauri::Error> {
    app.emit("job-progress", message)
}
```

## Frontend Listen

```ts
import { listen } from "@tauri-apps/api/event";

const unlisten = await listen<string>("job-progress", (event) => {
  console.log(event.payload);
});

unlisten();
```

Clean up listeners when components unmount or effects re-run.

## Command IPC

This project should use generated command bindings.

```ts
const result = await commands.scanCompanies(basePath, periodFolder);
```

Avoid raw command names because they bypass TypeScript binding checks and drift more easily when Rust signatures change.

## Large Data

For large or incremental payloads, avoid returning a giant JSON value from one command if it can cause UI stalls. Consider chunking, events, channels, or file-based handoff depending on UX needs.
