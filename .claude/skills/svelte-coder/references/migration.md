# Migration — Svelte 4 → 5

## Key Syntax Changes

| Svelte 4 | Svelte 5 |
|----------|----------|
| `let count = 0` | `let count = $state(0)` |
| `$: doubled = count * 2` | `let doubled = $derived(count * 2)` |
| `$: { console.log(count); }` | `$effect(() => { console.log(count); })` |
| `export let name` | `let { name } = $props()` |
| `<slot />` | `{@render children()}` |
| `<slot name="header" />` | `{@render header?.()}` |
| `createEventDispatcher` | Callback props (`onchange`, `onsubmit`) |
| `on:click` | `onclick` |
| `<svelte:component this={Comp} />` | `<Comp />` |
| `$$props`, `$$restProps` | `let { known, ...rest } = $props()` |

## What Breaks

### createEventDispatcher — Removed

```svelte
<!-- Svelte 4 -->
<script>
  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher();
</script>
<button on:click={() => dispatch('close', { reason: 'user' })}>

<!-- Svelte 5 -->
<script>
  let { onclose } = $props();
</script>
<button onclick={() => onclose?.({ reason: 'user' })}>
```

### Slots — Replaced by Snippets

```svelte
<!-- Svelte 4 -->
<slot name="header" />
<slot />

<!-- Svelte 5 -->
<script>
  import type { Snippet } from 'svelte';
  interface Props {
    header?: Snippet;
    children: Snippet;
  }
  let { header, children }: Props = $props();
</script>
{#if header}{@render header()}{/if}
{@render children()}
```

### Reactivity — Explicit via Runes

```svelte
<!-- Svelte 4 — implicit reactivity -->
<script>
  let count = 0;
  $: doubled = count * 2;
  $: console.log(count);
</script>

<!-- Svelte 5 — explicit reactivity -->
<script>
  let count = $state(0);
  let doubled = $derived(count * 2);
  $effect(() => { console.log(count); });
</script>
```

## Migration Command

```bash
npx sv migrate svelte-5
```

## Svelte 4 Compatibility

Svelte 5 is **backward compatible**:
- Old Svelte 4 components continue to work alongside Svelte 5 components
- Gradually migrate as needed — no big-bang rewrite required
- Mixed mode is fully supported

## Common Migration Issues

### 1. Forgetting $state after migration

```svelte
<!-- After auto-migration, verify all state uses $state -->
<script>
  let count = $state(0); // ✅ Verify this exists
</script>
```

### 2. Event handler syntax

```svelte
<!-- Svelte 4 -->
<button on:click={handler}>

<!-- Svelte 5 -->
<button onclick={handler}>
```

### 3. Store compatibility

Svelte 5 still supports stores (`writable`, `readable`), but runes are preferred for new code:

```svelte
<script>
  // Still works
  import { writable } from 'svelte/store';
  const count = writable(0);

  // Preferred in Svelte 5
  let count = $state(0);
</script>
```

### 4. Class directives

```svelte
<!-- Svelte 4 -->
<div class:active={isActive}>

<!-- Svelte 5 — both work, but classList is preferred -->
<div class:active={isActive}>
<div classlist={{ active: isActive }}>
```
