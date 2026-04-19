---
name: svelte-coder
description: Complete Svelte 5 coding guide for this monorepo. Use when writing Svelte components, pages, or any .svelte/.svelte.ts code. Covers runes ($state, $derived, $effect, $props, $bindable), event handling, component patterns, and project-specific conventions. FORBIDS Svelte 3/4 legacy syntax (export let, $:, beforeUpdate, afterUpdate, onDestroy, createEventDispatcher).
---

# Svelte Coder - Svelte 5 Runes-Only Guide

All `.svelte` files in this project MUST use Svelte 5 runes. No legacy syntax allowed.

## Forbidden Patterns (Hard Blocks)

```svelte
<!-- ❌ NEVER — use $props() instead -->
<script lang="ts">
  export let name: string;
</script>

<!-- ❌ NEVER — use $derived() or $effect() instead -->
<script lang="ts">
  $: doubled = count * 2;
  $: console.log(count);
</script>

<!-- ❌ NEVER — use $effect cleanup instead -->
<script lang="ts">
  import { onDestroy, beforeUpdate, afterUpdate } from 'svelte';
  onDestroy(() => cleanup());
  beforeUpdate(() => { ... });
  afterUpdate(() => { ... });
</script>

<!-- ❌ NEVER — use callback props instead -->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher();
</script>

<!-- ❌ NEVER — use onclick not on:click -->
<button on:click={handler}>  <!-- Svelte 4 -->
<button onclick={handler}>   <!-- Svelte 5 -->
```

## Core Runes Quick Reference

| Rune | Purpose | Replaces |
|------|---------|----------|
| `$props()` | Receive props | `export let` |
| `$state()` | Reactive state | implicit `let` |
| `$state.raw()` | Shallow state (large data) | N/A |
| `$derived()` | Computed values | `$:` declarations |
| `$derived.by()` | Complex computed | `$:` blocks |
| `$effect()` | Side effects + cleanup | `$:` + `onDestroy` |
| `$effect.pre()` | Pre-DOM effects | `beforeUpdate` |
| `$bindable()` | Two-way binding | N/A |
| `$inspect()` | Debug reactive state | N/A |

## Essential Patterns

### Props — `$props()`

```svelte
<script lang="ts">
  interface Props {
    title: string;
    count?: number;
    onChange: (value: number) => void;
    children: Snippet;
  }

  let { title, count = 0, onChange, children }: Props = $props();
</script>
```

### State — `$state()`

```svelte
<script lang="ts">
  // Deep reactive (default) — objects are deeply reactive via Proxy
  let user = $state({ name: 'Alice', address: { city: 'Taipei' } });
  user.address.city = 'Kaohsiung'; // triggers reactivity

  // Shallow reactive for large data you replace entirely
  let items = $state.raw<BigItem[]>([]);
  items = [...items, newItem]; // must reassign to trigger updates
</script>
```

### Derived — `$derived()`

```svelte
<script lang="ts">
  let items = $state([{ price: 10, qty: 2 }]);

  // Simple — use $derived
  let subtotal = $derived(items.reduce((s, i) => s + i.price * i.qty, 0));

  // Complex — use $derived.by
  let filtered = $derived.by(() => {
    if (filter === 'all') return items;
    return items.filter(i => i.status === filter);
  });
</script>
```

### Effects — `$effect()`

```svelte
<script lang="ts">
  // Auto-tracks dependencies, runs after DOM updates
  $effect(() => {
    console.log(`Count: ${count}`);
  });

  // With cleanup (replaces onDestroy)
  $effect(() => {
    const timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  });
</script>
```

### Snippets (Replace Slots)

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    children: Snippet;
    actions?: Snippet;
  }

  let { children, actions }: Props = $props();
</script>

{@render children()}
{#if actions}{@render actions()}{/if}
```

## Critical Pitfalls

### 1. Destructuring $state breaks reactivity

```svelte
<script lang="ts">
  let user = $state({ name: 'Alice' });
  const { name } = user; // ❌ Copies values, loses reactivity
  user.name;             // ✅ Access directly
</script>
```

### 2. Runes only work in `.svelte` and `.svelte.ts`

```typescript
// ❌ counter.ts — $state is undefined
let count = $state(0);
// ✅ counter.svelte.ts — runes work
let count = $state(0);
```

### 3. $effect infinite loops — never write to state you read

```svelte
<script lang="ts">
  $effect(() => { count = count + 1; }); // ❌ infinite loop
  let doubled = $derived(count * 2);     // ✅ use $derived
</script>
```

### 4. Async in $derived — impossible, use $effect + $state

```svelte
<script lang="ts">
  let user = $state<User | null>(null);
  $effect(() => { fetchUser(id).then(u => user = u); }); // ✅
</script>
```

### 5. $bindable required for two-way binding

```svelte
<script lang="ts">
  let { value = $bindable() } = $props(); // ✅ parent must use bind:value
</script>
```

## Project Conventions

1. **Always** `<script lang="ts">` — TypeScript mandatory
2. **Always** type `$props()` with `interface Props`
3. **Always** use keyed `{#each items as item (item.id)}`
4. **Prefer** `$derived()` over getter functions for computed values
5. **Use** `bind:value={}` for form inputs with `$state()`
6. **Use** `onMount` for initial data fetching (still valid in Svelte 5)
7. **Use** callback props (`onChange`, `onSubmit`) instead of `createEventDispatcher`
8. **Filename**: `kebab-case.svelte`

## Data Fetching Pattern

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import type { Customer } from '@novercont/interfaces';
  import { findAllCustomers } from '$lib/services/customers';

  let customers = $state<Customer[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      customers = await findAllCustomers();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load';
    } finally {
      loading = false;
    }
  });
</script>
```

## Form Pattern

```svelte
<script lang="ts">
  let form = $state({ name: '', email: '' });
  let errors = $state<Record<string, string>>({});

  function handleSubmit() {
    errors = {};
    if (!form.name.trim()) errors.name = 'Name required';
    if (!form.email.includes('@')) errors.email = 'Invalid email';
    if (Object.keys(errors).length > 0) return;
    submitForm(form);
  }
</script>

<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
  <Input bind:value={form.name} />
  {#if errors.name}<p class="text-destructive">{errors.name}</p>{/if}
  <Button type="submit">Submit</Button>
</form>
```

## Detailed References

- **$state() deep dive** (raw, snapshot, class fields, SvelteSet/SvelteMap) → `references/state.md`
- **$derived() vs $derived.by()** (performance, when to use which) → `references/derived.md`
- **$effect() pitfalls** (infinite loops, SSR, tracking, untrack) → `references/effects.md`
- **$props() & $bindable()** (optional props, rest spread, controlled vs uncontrolled) → `references/props.md`
- **Snippets** (render props, recursive, optional, as props) → `references/snippets.md`
- **Performance** (fine-grained reactivity, $state.raw, bundle size) → `references/performance.md`
- **Migration** (Svelte 4 → 5, what breaks, compatibility) → `references/migration.md`
