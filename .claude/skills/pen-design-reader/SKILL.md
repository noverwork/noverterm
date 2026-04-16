---
name: pen-design-reader
description: "Read and inspect .pen design files using the Pencil MCP tools. MUST use this skill whenever a .pen file is mentioned, referenced, or needs to be read — including when the user says 'read the design', 'check the UI', 'look at the design file', 'open ui-design.pen', or any reference to .pen files. .pen files are encrypted and CANNOT be read with Read, Grep, or cat — only Pencil MCP tools work."
---

# .pen Design File Reader

`.pen` files are encrypted design documents that can **only** be accessed through the Pencil MCP server tools. Standard file tools (Read, Grep, cat, head) will return garbage or errors.

## Critical Rule: Always Fresh Read

**每次需要讀取 .pen 設計檔時，必須重新透過 MCP 工具讀取。絕對不可依賴：**

- 對話上下文中先前讀取的結果
- Memory 中儲存的設計資訊
- 之前對話中看過的節點結構或截圖

設計檔隨時可能被修改，任何快取資料都可能過時。**每次都要重新走完整讀取流程。**

## When to activate

Any time the conversation involves a `.pen` file:
- User says "read ui-design.pen", "check the design", "look at the design file"
- You need to reference UI screens, components, or design tokens
- Building UI that should match a design spec in a `.pen` file
- Comparing implemented UI against the design source

## How to read .pen files

### Step 1: Get editor state

先確認目前編輯器狀態，了解是否已有開啟的 .pen 檔案：

```
mcp__pencil__get_editor_state()
```

如果沒有開啟目標檔案，使用 `open_document` 開啟：

```
mcp__pencil__open_document(filePathOrNew: "docs/ui-design.pen")
```

### Step 2: Discover top-level structure

取得頂層結構，列出所有畫面分類：

```
mcp__pencil__batch_get(patterns: [{}], searchDepth: 1, readDepth: 0)
```

這會列出如 `App`、`Desktop`、`Component`、`TabBar` 等頂層群組。

### Step 3: Enumerate ALL screens (Mobile + Desktop)

**必須同時遍歷 Mobile 和 Desktop 兩套畫面。** 不可只查一邊就停。

先列出 Mobile（App）底下所有畫面：

```
mcp__pencil__batch_get(patterns: [{"name": "App"}], searchDepth: 1, readDepth: 1)
```

再列出 Desktop 底下所有畫面：

```
mcp__pencil__batch_get(patterns: [{"name": "Desktop"}], searchDepth: 1, readDepth: 1)
```

也列出共用 Component：

```
mcp__pencil__batch_get(patterns: [{"name": "Component"}], searchDepth: 1, readDepth: 1)
```

**完整掃描後，才能確定哪些畫面與目標相關。** 如果目標畫面不在預期名稱中，繼續往下搜尋其他頂層節點，直到找到為止。

### Step 4: Deep-read target screens (both versions)

找到目標畫面後，**同時讀取 Mobile 和 Desktop 兩個版本的完整結構**：

```
mcp__pencil__batch_get(nodeIds: ["mobileScreenId", "desktopScreenId"], readDepth: 3)
```

如果節點樹太大導致溢出，分批讀取：先 `readDepth: 1` 看子節點清單，再針對需要的子節點逐一深讀。

### Step 5: Visual verification (both versions)

**必須對 Mobile 和 Desktop 版本都取截圖：**

```
mcp__pencil__get_screenshot(nodeId: "mobileScreenId")
mcp__pencil__get_screenshot(nodeId: "desktopScreenId")
```

截圖是驗證設計的關鍵——節點資料無法完整反映視覺效果。實作 UI 前必須看過截圖。

### Step 6: Read design tokens (if needed)

提取色彩變數、間距、主題資料：

```
mcp__pencil__get_variables()
```

### Step 7: Read related components

如果畫面中使用了可複用元件，也要讀取它們的結構：

```
mcp__pencil__batch_get(patterns: [{"reusable": true}], searchDepth: 3, readDepth: 2)
```

## Common patterns for this project

The design file `docs/ui-design.pen` is organized as:

- **`App/*`** — Mobile screens (402×874px)
- **`Desktop/*`** — Desktop screens (1200×800px)
- **`Component/*`** — Reusable UI components
- **`Desktop/Sidebar`** — Collapsed sidebar (72px)
- **`Desktop/SidebarExpanded`** — Expanded sidebar (220px)
- **`TabBar`** — Mobile bottom navigation

**每個頁面通常同時存在於 `App/` 和 `Desktop/` 底下，名稱可能略有不同（如 `App/Dashboard` vs `Desktop/Dashboard`）。實作任何頁面都必須找到並參考兩個版本。**

## Rules

- **NEVER** use `Read`, `Grep`, `cat`, or any file tool on `.pen` files
- **NEVER** rely on previously read results from context or memory — always re-read from the file
- **ALWAYS** use `mcp__pencil__batch_get` to read structure and data
- **ALWAYS** enumerate all top-level groups and scan through all screens before concluding a target screen doesn't exist
- **ALWAYS** read both Mobile (`App/*`) and Desktop (`Desktop/*`) versions of the target screen
- **ALWAYS** use `mcp__pencil__get_screenshot` on both versions to visually verify before implementing
- When the result is too large (exceeds token limit), narrow your search with `parentId`, tighter `patterns`, or lower `readDepth` — but do NOT skip screens
- Combine multiple searches into a single `batch_get` call when possible (multiple patterns, multiple nodeIds)
- If a screen is not found under expected names, keep searching other top-level nodes — do not assume it doesn't exist
