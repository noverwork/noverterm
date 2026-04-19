# Performance

## Fine-Grained Reactivity

Svelte 5 uses signals — only changed parts update:

```svelte
<script lang="ts">
  let items = $state([1, 2, 3, 4, 5]);
  let filtered = $derived(items.filter(x => x > 2));
</script>

<ul>
  {#each filtered as item}
    <li>{item}</li> <!-- Only the specific text node updates -->
  {/each}
</ul>
```

## Performance Tips

### 1. Use $state.raw for large data

```svelte
<script lang="ts">
  // ❌ Deep proxy on 10,000 items — slow initialization
  let items = $state<BigItem[]>(apiResponse);

  // ✅ Shallow — fast, reassign when needed
  let items = $state.raw<BigItem[]>(apiResponse);
  items = [...items, newItem]; // must reassign
</script>
```

### 2. Prefer $derived over functions for computed values

```svelte
<script lang="ts">
  let items = $state([{ price: 10, qty: 2 }]);

  // ❌ Recalculates every render
  function getSubtotal() {
    return items.reduce((sum, i) => sum + i.price * i.qty, 0);
  }

  // ✅ Memoized — only recalculates when items change
  let subtotal = $derived(items.reduce((sum, i) => sum + i.price * i.qty, 0));
</script>
```

### 3. Use SvelteSet/SvelteMap for reactive collections

```svelte
<script lang="ts">
  import { SvelteSet, SvelteMap } from 'svelte/reactivity';

  // ❌ Regular Set — .add() won't trigger updates
  let selected = $state(new Set<string>());

  // ✅ SvelteSet — .add() triggers updates
  let selected = $state(new SvelteSet<string>());
</script>
```

### 4. Avoid unnecessary effects

Each `$effect` has overhead. Don't use effects for things that can be derived:

```svelte
<script lang="ts">
  // ❌ Effect overhead for computation
  let doubled = $state(0);
  $effect(() => { doubled = count * 2; });

  // ✅ Zero overhead, lazy evaluation
  let doubled = $derived(count * 2);
</script>
```

### 5. Use {#key} blocks to force re-renders

```svelte
<script lang="ts">
  let tab = $state('overview');
</script>

{#key tab}
  <Component /> <!-- Forces full re-mount when tab changes -->
{/key}
```

### 6. Keyed {#each} for list performance

```svelte
<!-- ✅ Unique key — efficient DOM reuse -->
{#each items as item (item.id)}
  <ListItem {item} />
{/each}

<!-- ❌ Index key — destroys/recreates on reorder -->
{#each items as item, i (i)}
  <ListItem {item} />
{/each}
```

## Bundle Size

Svelte 5 produces smaller bundles than Svelte 4:
- Runtime: ~1.6KB gzipped (vs React 42KB, Vue 22KB)
- Typical reduction: ~50% for existing apps migrating from Svelte 4
