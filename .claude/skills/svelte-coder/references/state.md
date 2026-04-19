# $state() Deep Dive

## Deep Reactivity (Default)

`$state()` creates a Proxy that tracks nested mutations automatically:

```svelte
<script lang="ts">
  let user = $state({
    profile: {
      address: { city: 'NYC' }
    }
  });

  // This triggers updates — no reassignment needed
  user.profile.address.city = 'Boston';
</script>
```

## $state.raw — Opting Out of Deep Reactivity

Use `$state.raw` for large data structures where you always replace the entire value:

```svelte
<script lang="ts">
  let bigData = $state.raw([]);

  // ✅ This works — reassignment triggers update
  function addItem(item: Item) {
    bigData = [...bigData, item];
  }

  // ❌ This does NOT trigger updates — mutations ignored
  bigData.push(item);
</script>
```

**When to use `$state.raw`:**
- Large arrays where you only replace the whole array (API responses)
- Objects that are completely replaced, not mutated
- Class instances that manage their own state
- Data from external stores that you don't mutate in-place

## $state.snapshot()

Converts reactive state to a plain JavaScript object (useful for sending to APIs):

```svelte
<script lang="ts">
  let form = $state({ name: '', email: '' });

  async function submit() {
    const plain = $state.snapshot(form); // { name: '', email: '' }
    await api.submit(plain);
  }
</script>
```

**Gotcha with class fields:** `$state.snapshot()` uses `.toJSON()` which can't see private fields created by `$state` in classes. Use getters instead:

```svelte
<script lang="ts">
  class FormStore {
    name = $state('');
    email = $state('');

    toJSON() {
      return { name: this.name, email: this.email };
    }
  }
</script>
```

## SvelteSet and SvelteMap

For reactive Sets and Maps, use the special collections:

```svelte
<script lang="ts">
  import { SvelteSet, SvelteMap } from 'svelte/reactivity';

  let selectedIds = $state(new SvelteSet<string>());
  let metadata = $state(new SvelteMap<string, Meta>());

  // These trigger updates:
  selectedIds.add('item-1');
  metadata.set('item-1', { status: 'active' });
</script>
```

Regular `Set` and `Map` inside `$state()` won't track `.add()`, `.delete()`, `.set()` mutations.

## Class Fields with $state

```svelte
<script lang="ts">
  class Counter {
    count = $state(0);

    // Auto-generated getter/setter — access via instance
    increment() { this.count++; }
  }

  let counter = new Counter();
  counter.count; // works via generated getter
</script>
```

## Common Mistakes

### Destructuring breaks reactivity

```svelte
<script lang="ts">
  let user = $state({ name: 'Alice', age: 30 });

  // ❌ Destructuring copies values, loses Proxy tracking
  const { name, age } = user;

  // ✅ Access properties directly
  user.name
</script>
```

### Forgetting $state altogether

```svelte
<script lang="ts">
  let count = 0;        // ❌ Not reactive
  let count = $state(0); // ✅ Reactive
</script>
```

### Using runes in plain .ts files

```typescript
// ❌ counter.ts — $state is undefined at runtime
let count = $state(0);

// ✅ counter.svelte.ts — compiler transforms runes
let count = $state(0);
```

### Mutating state in $effect causes infinite loops

```svelte
<script lang="ts">
  // ❌ Reads count, writes count → infinite loop
  $effect(() => { count = count + 1; });

  // ✅ Use $derived for computed values
  let doubled = $derived(count * 2);
</script>
```
