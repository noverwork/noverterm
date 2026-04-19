# $derived() vs $derived.by()

## When to Use Which

| Scenario | Use |
|----------|-----|
| Simple expressions, single-line | `$derived(count * 2)` |
| Multiple operations, conditionals, loops | `$derived.by(() => { ... })` |
| Need to call as a function with args | Regular function (not derived) |

## $derived — Simple Expressions

```svelte
<script lang="ts">
  let count = $state(0);
  let doubled = $derived(count * 2);

  // Chain derivations
  let quadrupled = $derived(doubled * 2);

  // From arrays
  let items = $state([{ price: 10, qty: 2 }]);
  let subtotal = $derived(items.reduce((s, i) => s + i.price * i.qty, 0));
  let tax = $derived(subtotal * 0.05);
  let grandTotal = $derived(subtotal + tax);
</script>
```

## $derived.by — Complex Logic

```svelte
<script lang="ts">
  let items = $state<Item[]>([]);
  let filter = $state<'all' | 'active' | 'done'>('all');

  let filtered = $derived.by(() => {
    if (filter === 'all') return items;
    return items.filter(i => i.status === filter);
  });

  let summary = $derived.by(() => {
    const total = items.length;
    const active = items.filter(i => i.status === 'active').length;
    const done = items.filter(i => i.status === 'done').length;
    return { total, active, done };
  });
</script>
```

## Performance

Both `$derived` and `$derived.by` are **lazy** — they only recompute when dependencies change. The difference is ergonomics, not performance.

```svelte
<script lang="ts">
  let items = $state([1, 2, 3, 4, 5]);

  // Option 1: $derived — memoized, auto-tracks deps
  let filtered = $derived(items.filter(x => x > 1));

  // Option 2: Regular function — recalculates every render
  function getFiltered() {
    return items.filter(x => x > 1);
  }
</script>
```

**Rule of thumb:** Use `$derived` when the value is used in multiple places or is expensive to compute. Use regular functions for simple, single-use transformations.

## Pitfalls

### Mutating derived values — impossible

```svelte
<script lang="ts">
  let count = $state(0);
  let doubled = $derived(count * 2);

  // ❌ Derived values are read-only
  doubled = 10; // Error or silently ignored
</script>
```

### Async in $derived — not possible

```svelte
<script lang="ts">
  let userId = $state(1);

  // ❌ $derived can't handle async
  let user = $derived(await fetchUser(userId));

  // ✅ Use $effect + $state
  let user = $state<User | null>(null);
  $effect(() => {
    fetchUser(userId).then(u => user = u);
  });
</script>
```

### Callbacks inside $derived.by don't track

```svelte
<script lang="ts">
  let items = $state([1, 2, 3]);

  // ❌ setTimeout callback won't track dependencies
  let futureItems = $derived.by(() => {
    setTimeout(() => items, 1000);
    return items;
  });

  // ✅ Keep derived synchronous
  let doubled = $derived(items.map(x => x * 2));
</script>
```

### Over-using $derived.by when $derived suffices

```svelte
<script lang="ts">
  let count = $state(0);

  // ❌ Unnecessary complexity
  let doubled = $derived.by(() => count * 2);

  // ✅ Simple is better
  let doubled = $derived(count * 2);
</script>
```
