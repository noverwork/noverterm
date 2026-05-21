# DESIGN-PRINCIPLES.md — 抗 AI 味設計工作流

這份文件定義 **Truley Sentinel 做設計/實作 UI 時的流程守則**，對人與 AI 一體適用。

> **為什麼需要這份文件？**
> 「AI 味」的本質是**脈絡不足時模型會退回訓練資料的平均值** — 那個平均值就是近年 Dribbble + SaaS landing page 的中位數。
> 避免 AI 味的唯一方法是**事前塞滿脈絡、事中強制 reuse、事後 self-audit**，不是事後過濾。
>
> 這份文件是 process；tokens 與 component spec 在 [`DESIGN.md`](./DESIGN.md)；實作好的 widgets 在 [`packages/app/lib/shared/widgets/`](./packages/app/lib/shared/widgets/)。

---

## A. 硬規則（MUST / MUST NOT）

違反 = PR 被打回。

### A1. 動工前必做（三步驟）

> 每次新畫面、新元件、redesign 既有畫面前，**依序走完這三步**。跳步 = 違反。

1. **讀 `DESIGN.md`** — 確認可用的 tokens（color / type / spacing / radius）。
2. **列 widget inventory** — 掃 `packages/app/lib/shared/widgets/`，寫下：
   - 會直接使用哪些 widget
   - 會改哪些 widget 的 variant（並說明現有 variant 為何不夠）
   - 要新增哪個 widget（並說明為何不能用既有的擴充）
3. **Verbalize the visual system** — 動手寫 code 前，用文字先講清楚：
   - 要用哪幾層 heading（`headingTitle` / `headingLg` / ...）
   - 要用哪個 accent（`accentBlue` for action、`accentGreen` for success、...）
   - 主 radius pattern（card `rounded-[1.35rem]`、button `rounded-2xl`、pill 31）
   - spacing 節奏（page 16、section 12–16、card padding 16、icon gap 8）
   - **Card style 比照 connections-view**（flat border + subtle bg，非 gradient + shadow）

> 人在工作時，把 1–3 寫在 PR description 開頭或 task comment 裡。
> AI 在協作時，在動手寫 code 前把 1–3 輸出給使用者審過。

### A2. 反面清單（MUST NOT）

這些是 AI 最常退回的「訓練集平均值」，也是這個專案刻意不走的方向：

| # | MUST NOT | 為什麼 |
|---|---|---|
| 1 | 使用漸層背景（`LinearGradient`, `RadialGradient`, CSS gradient） | `DESIGN.md` 無 gradient token，出現 = 在發明東西 |
| 2 | 用 emoji 當 icon | 已指定 Lucide 為 icon set |
| 3 | 使用 Material default colors（`Colors.blue`, `Colors.grey`, ...） | 明文禁止於 `DESIGN.md` |
| 4 | 使用 raw hex / `Color(0x...)` 於 widget code | 必用 `context.colors.xxx` |
| 5 | 用 `AppTypography` style 時把 color 寫死在定義 | style 永遠 colorless，color 走 `.copyWith(color: ...)` |
| 6 | 做「左邊 accent 彩條」的 list row | 使用既有的 `StatusBadge` 表達狀態 |
| 7 | 加裝飾性 UI（浮動粒子、呼吸 glow、多層陰影光暈、「科技感」裝飾） | 錄音狀態用既有 `shadowOverlay` token，不要再加 |
| 8 | 加 data slop（為湊版面放假統計、假進度條、無意義 icon） | 空間留白 > 填充，少即是多 |
| 9 | 新增 widget，當 90% 需求可由既有 shared widget 的 variant 滿足 | 先改 variant、先在 PR 討論，不要複製一份 |
| 10 | 新增 design token（color / spacing / radius / font） | 新 token 必須先在 PR 討論 |
| 11 | 用 inline SVG 硬畫插圖 | 放 placeholder，要求真實素材 |
| 12 | 讓 padding/margin 逃出 spacing scale `{4, 8, 12, 16, 24, 32, 48}` | `DESIGN.md` 已明訂 |
| 13 | 讓 radius 逃出 radius scale `{8, 12, 14, 16, 31}` 或專案標準 `rounded-[1.35rem]` | `DESIGN.md` 已明訂；card/empty/form 用 `rounded-[1.35rem]`（connections-view 基準） |
| 14 | 跳過 `HoverBuilder` 自寫 hover 邏輯 | 所有桌面端可點元素必用 `HoverBuilder` 包裹 |

---

## B. 軟原則（工作流建議）

不是硬規定，但跳過這些你大機率會產出平均值作品。

### B1. Reference before designing
新畫面前先到 [`docs/design-references/`](./docs/design-references/) 翻一輪，或明確指定「這個 flow 參考 {app}/{screen}」。

**不要**從形容詞出發（"modern"、"clean"、"elegant"）。形容詞 = 脈絡為零 = AI 會回到平均值。

### B2. Widget inventory first
動工前把 A1-2 的 inventory 寫出來。這一步不超過 5 分鐘，但能攔下 80% 的「重造輪子」衝動。

### B3. Verbalize before coding
先用文字講系統再動手。這一步讓自己或別人能在動工前審過方向，比起事後 review 成本低 10 倍。

### B4. Challenge the first output
AI 第一版通常是平均值。如果產出看起來像「Material default + 一點品牌色」，**丟掉重來 + 加更多約束**，不要在第一版上修。
「讓 AI 不像 AI」的唯一方法是不給它回到平均值的機會。

### B5. Cross-category references
錄音類 app 不要只看錄音類 app。氣質要從跨類別優秀產品盜取：
- **文字/閱讀**：Bear、Craft、Things
- **生產力**：Linear、Height、Superhuman
- **資訊密度**：Stripe Docs、Notion、Raycast
- **情感溫度**：Day One、Overcast

看錄音 app 只用來**避免套路**（參見 `docs/design-references/same-category/`）。

---

## C. 完工前自審 Checklist（強制）

> **觸發時機**：每次 UI 實作或修改**宣告完成之前**（包含 AI 回覆「完成」前、自己 commit 前）必須走完這 7 條。不是 code review 階段的關卡，是**實作階段的關卡**。

### AI 協作時

AI 在回報「UI 完成」之前，**必須在該回合的結尾逐條輸出** pass / fail 判定 + 若 fail 的具體證據（哪個檔案、哪一行）。

格式：

```markdown
### Self-Audit (DESIGN-PRINCIPLES §C)
- [x] 色彩全走 `context.colors.xxx` — 無 raw hex
- [x] 文字 style 全走 `AppTypography`，color via `.copyWith()`
- [x] padding/margin 都在 spacing scale `{4, 8, 12, 16, 24, 32, 48}`
- [x] radius 都在 radius scale `{8, 12, 14, 16, 31}` 或專案標準 `rounded-[1.35rem]`（card / empty state / form）
- [x] 無漸層 / emoji icon / 裝飾性 glow（card 用 flat border 模式，參照 §E5）
- [x] 無湊版面的假資料、假統計、無意義 icon
- [x] 新增/修改的 widget 追溯到既有 shared widget 模式
- [x] card / icon / button style 比照 connections-view（§E），非 snippets-view
```

任何一條 fail → **當場改完再宣告完成**，不要留著下一回合處理。跳過 audit 直接宣告完成 = 規範失效。

### 自己寫時

每個 UI 改動在 commit 前自己過一遍。發現 fail → 當下修，不要寄望之後會回頭看。

### 例外

A2 反面清單的第 10 條（新增 token）或任何已在 task 開頭明文豁免的項目，可以在對應的 checkbox 後註明 `(exempt: <reason>)`。但「忘了」「時間不夠」**不是** 合法例外。

---

## D. Reference 素材庫

位置：[`docs/design-references/`](./docs/design-references/)

### 何時要來翻
- 任何新畫面動工前
- 現有畫面 redesign 前
- 做 variant / 視覺探索時

### 資料夾結構
- `same-category/` — 錄音/語音筆記類 app 截圖（**警示用**：避免套路）
- `cross-category/` — 跨類別氣質參考（Things, Craft, Bear, Linear, Stripe...）
- `type-hierarchy/` — 只看字型層級的截圖（editorial, docs, IDE）
- `empty-states/` — 空狀態專題（最常被做爛）
- `onboarding/` — 引導流程專題

### 命名慣例
`{app}-{screen}.png` — e.g. `bear-settings.png`、`things-today-empty.png`。

### 加素材時
遇到靈感就丟進去。每張截圖旁可放 `{filename}.notes.md` 記「為什麼放這張、要參考哪個部分、要避免什麼」。
反面素材也要收 — 在 `same-category/` 放 AI 味重的截圖，檔名前綴 `AVOID-` 標記。

---

## E. Gold Standard — Connections Page

`connections-view.svelte` 與 `connection-form.svelte` 是本專案 list/form 畫面的視覺基準。
新頁面、新元件應 **優先比照此模式**，而非自行發明。

### E1. Page Shell（所有 list view 共用）

```html
<div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
  <section class="ide-panel flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-white sm:p-6">
```

- 外層 `workspace-canvas` + 內層 `ide-panel` 是固定組合
- Page padding: `px-5 py-6 lg:px-8`
- Section padding: `p-5 sm:p-6`

### E2. Page Header

```html
<div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
  <div>
    <p class="section-title text-cyan-200/70">{Category}</p>
    <h1 class="mt-2 text-2xl font-semibold tracking-tight">{Page Title}</h1>
    <p class="mt-2 text-sm text-slate-500">{One-line description.}</p>
  </div>
  <Button ... class="gap-2 self-start rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200">
    <Plus class="size-3.5" /> Add {item}
  </Button>
</div>
```

- section-title: `text-cyan-200/70`（小字分類標籤）
- 標題: `text-2xl font-semibold tracking-tight`
- 副標題: `text-sm text-slate-500`
- 分隔線: `border-b border-white/10 pb-5`
- Primary button: `rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200`

### E3. Error Banner

```html
<div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
```

- 固定 `rounded-2xl`，不用 `rounded-xl`

### E4. Empty State

```html
<div class="flex h-full min-h-[16rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground">
```

- `rounded-[1.35rem]` 是本專案 empty state 的標準 radius
- `border-dashed border-white/10`
- `bg-white/[0.025]`
- `min-h-[16rem]`

### E5. List Card（card 基準）

```html
<article class="group rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-white/14 hover:bg-white/[0.055]">
```

- **Card radius**: `rounded-[1.35rem]`（≈21.6px，這是本專案 card 的標準值）
- **Border**: `border-white/8` default → `hover:border-white/14`
- **Background**: `bg-white/[0.03]` default → `hover:bg-white/[0.055]`
- **No gradient, no shadow** — 純色 + border 做層級
- Padding: `px-4 py-4`
- Grid: `grid gap-3 px-1 py-5 md:grid-cols-2 xl:grid-cols-3`

### E6. Card Icon

```html
<div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
  <Icon class="size-5" />
</div>
```

- Icon container: `size-10` + `rounded-2xl`
- Border: `border-cyan-300/14`
- Background: `bg-cyan-300/8`（極淡 cyan）
- Text: `text-cyan-200`
- **No glow shadow** — 不用 `shadow-[0_0_24px_...]`

### E7. Card Action Buttons

```html
<Button variant="ghost" size="xs" class="gap-1.5 rounded-xl bg-white/[0.035] text-slate-200 hover:bg-cyan-300/10 hover:text-white">
<Button variant="ghost" size="xs" class="gap-1.5 rounded-xl text-slate-400 hover:bg-white/7 hover:text-white">
<Button variant="ghost" size="xs" class="gap-1.5 rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300">
```

- Primary action: `bg-white/[0.035]` base → `hover:bg-cyan-300/10`
- Secondary action: transparent base → `hover:bg-white/7`
- Destructive: transparent base → `hover:bg-red-400/10 hover:text-red-300`
- Button radius: `rounded-xl`

### E8. Tab Bar（filter tabs）

```html
<button class="border-b-2 border-cyan-300/40 px-1 pb-2 pt-2 text-sm font-medium text-white">  <!-- active -->
<button class="border-b-2 border-transparent px-1 pb-2 pt-2 text-sm font-medium text-slate-400 transition hover:text-white">  <!-- inactive -->
```

- Active: `border-cyan-300/40` + `text-white`
- Inactive: `border-transparent` + `text-slate-400` + `hover:text-white`
- Badge: `rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] text-slate-400`

### E9. Form Container

```html
<form class="mt-5 rounded-[1.35rem] border border-cyan-300/24 bg-cyan-300/8 p-5 shadow-[0_16px_42px_rgb(34_211_238/0.08)]">
```

- Form 外框: `rounded-[1.35rem] border border-cyan-300/24 bg-cyan-300/8`
- Cyan shadow: `shadow-[0_16px_42px_rgb(34_211_238/0.08)]`（極淡，僅用於 form 聚焦）
- Section card: `rounded-2xl border border-white/8 bg-white/[0.035] p-4`
- Section icon: `rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200`

### E10. Input Fields

```html
<Input class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40" />
```

- Default: `border-white/10 bg-black/20`
- Focus: `focus-visible:border-cyan-300/40`
- Error: `border-destructive`

### E11. Context Menu / Dropdown

```html
<div class="fixed z-[9999] min-w-[14rem] rounded-xl border border-white/12 bg-[#0d1117] p-1 shadow-2xl shadow-black/50">
```

- Background: `bg-[#0d1117]`（GitHub dark 色）
- Border: `border-white/12`
- Shadow: `shadow-2xl shadow-black/50`
- Item radius: `rounded-lg`
- Active item: `bg-cyan-300/10 text-cyan-100`
- Hover item: `hover:bg-white/6`

### E12. 與 snippets-view 的差異（刻意不跟進）

snippets-view 使用了以下 pattern，**不在 gold standard 內**：
- `bg-gradient-to-b from-white/[0.07] to-white/[0.025]` — 漸層背景（違反 A2-1）
- `shadow-[0_18px_50px_rgb(0_0_0/0.16)]` — 重 shadow
- `rounded-[1.45rem]` — 與 card 標準 `rounded-[1.35rem]` 不一致
- Icon glow: `shadow-[0_0_24px_rgb(34_211_238/0.08)]` — 裝飾性 glow（違反 A2-7）

新頁面應 follow connections-view 的 flat + border 模式，而非 snippets-view 的 gradient + shadow 模式。

---

## 何時放寬這些規則

- **實驗性 spike** — 探索全新互動（非既有畫面改版）時，A1-2 的反面清單可暫時豁免，但 spike 結果要回到規則內才 merge。
- **A/B 視覺測試** — 刻意做離經叛道的版本測用戶反應，要明文標 `// spike: anti-pattern on purpose` 註解。

除此之外一律照走。
