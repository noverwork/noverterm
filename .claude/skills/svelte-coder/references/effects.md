# $effect() Pitfalls

## When to Use $effect

- DOM manipulation (third-party libraries like charts, maps)
- Logging/analytics
- Synchronizing with external systems (localStorage, WebSockets)
- **NOT for deriving values** — use `$derived` instead

## Basic Usage

```svelte
<script lang="ts">
  let count = $state(0);

  // Runs when count changes, after DOM updates
  $effect(() => {
    console.log('Count is now:', count);
  });

  // With cleanup
  $effect(() => {
    const controller = new AbortController();
    fetch('/api/data', { signal: controller.signal })
      .then(r => r.json())
      .then(data => /* ... */);

    return () => controller.abort(); // cleanup on re-run or destroy
  });
</script>
```

## $effect vs $effect.pre

| Rune | When It Runs | Use Case |
|------|--------------|----------|
| `$effect` | After DOM updates | Logging, external sync |
| `$effect.pre` | Before DOM updates | DOM measurements, scroll position |

```svelte
<script lang="ts">
  // For measuring DOM elements (needs updated dimensions)
  $effect.pre(() => {
    const height = element.clientHeight;
  });

  // For logging (after render complete)
  $effect(() => {
    console.log('Rendered!');
  });
</script>
```

## Critical Pitfalls

### Infinite Loops

```svelte
<script lang="ts">
  let count = $state(0);

  // ❌ Writing to state that's also read → infinite loop
  $effect(() => {
    count = count + 1;
  });
</script>
```

**Fix:** Use `$derived` or `untrack`:

```svelte
<script lang="ts">
  import { untrack } from 'svelte';

  let count = $state(0);

  // ✅ Use untrack to prevent dependency tracking
  $effect(() => {
    count = untrack(() => count) + 1;
  });
</script>
```

### Using $effect for Derived Values

```svelte
<script lang="ts">
  let firstName = $state('John');
  let lastName = $state('Doe');

  // ❌ Should use $derived
  let fullName = $state('');
  $effect(() => {
    fullName = `${firstName} ${lastName}`;
  });

  // ✅ Use $derived
  let fullName = $derived(`${firstName} ${lastName}`);
</script>
```

### Forgetting Dependencies Are Auto-Tracked

```svelte
<script lang="ts">
  let count = $state(0);
  let name = $state('Alice');

  // Only runs when count changes — name is NOT tracked
  $effect(() => {
    console.log(count);
  });
</script>
```

### $effect Only Runs in Browser (Not SSR)

```svelte
<script lang="ts">
  // This only runs in browser, not during SSR
  $effect(() => {
    console.log(window.innerWidth); // Safe — only runs client-side
  });
</script>
```

### Nested State Mutation Bug (Svelte 5.24+)

In some versions, mutating nested state inside an effect that also reassigns the root can cause missed updates:

```svelte
<script lang="ts">
  let state = $state({ value: 1 });

  $effect(() => {
    console.log(state.value);
    if (state.value == 1) {
      state = { value: 10 }; // reassignment
    }
  });

  function set100() {
    state.value = 100; // May not re-run effect in some versions
  }
</script>
```

**Fix:** Prefer reassignment over mutation when mixing both patterns, or upgrade to latest Svelte 5.

## SSR Considerations

```svelte
<script lang="ts">
  import { browser } from '$app/environment';

  // $effect only runs client-side
  $effect(() => {
    // Safe to access window, document, etc.
    const width = window.innerWidth;
  });

  // If you need data during SSR, use load functions
</script>
```
