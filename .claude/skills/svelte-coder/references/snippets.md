# Snippets

## Defining Snippets

```svelte
<script lang="ts">
  // Basic snippet
  {#snippet hello()}
    <span>Hello World</span>
  {/snippet}

  // With parameters
  {#snippet greeting(name: string)}
    <span>Hello, {name}!</span>
  {/snippet}

  // Rendering
  {@render hello()}
  {@render greeting('Alice')}
</script>
```

## Snippets as Props (Replacing Slots)

```svelte
<!-- Card.svelte -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    children: Snippet;
    footer?: Snippet;
  }

  let { title, children, footer }: Props = $props();
</script>

<div class="card">
  <h2>{title}</h2>
  <div class="card-body">
    {@render children()}
  </div>
  {#if footer}
    <div class="card-footer">
      {@render footer()}
    </div>
  {/if}
</div>

<!-- Usage -->
<Card title="My Card">
  {#snippet children()}
    <p>Card content here</p>
  {/snippet}

  {#snippet footer()}
    <button>Action</button>
  {/snippet}
</Card>
```

## Optional Snippets

```svelte
<script lang="ts">
  let { children } = $props();
</script>

<!-- Option 1: Optional chaining -->
{@render children?.()}

<!-- Option 2: Conditional with fallback -->
{#if children}
  {@render children()}
{:else}
  <p>Default content</p>
{/if}
```

## Render Props Pattern

```svelte
<!-- DataProvider.svelte -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    data: User | null;
    loading: boolean;
    children: Snippet<[user: User]>;
  }

  let { data, loading, children }: Props = $props();
</script>

{#if loading}
  <p>Loading...</p>
{:else if data}
  {@render children(data)}
{/if}

<!-- Usage -->
<DataProvider {data} {loading}>
  {#snippet children(user)}
    <p>{user.name}</p>
  {/snippet}
</DataProvider>
```

## Recursive Snippets

```svelte
{#snippet countdown(n: number)}
  {#if n > 0}
    <span>{n}...</span>
    {@render countdown(n - 1)}
  {:else}
    <span>Done!</span>
  {/if}
{/snippet}

{@render countdown(5)}
```

## Snippets in {#each}

```svelte
<script lang="ts">
  let items = $state([
    { id: 1, name: 'Alice' },
    { id: 2, name: 'Bob' },
  ]);
</script>

{#each items as item (item.id)}
  {@render row(item)}
{/each}

{#snippet row(item: { id: number; name: string })}
  <tr>
    <td>{item.id}</td>
    <td>{item.name}</td>
  </tr>
{/snippet}
```

## Pitfalls

### Snippets can't have state

```svelte
<!-- ❌ Can't define state inside snippets -->
{#snippet bad()}
  let count = $state(0); <!-- Won't work! -->
{/snippet}
```

### Forgetting to call snippet with ()

```svelte
{#snippet mySnippet()}
  <p>Content</p>
{/snippet}

<!-- ❌ Forgot () -->
{@render mySnippet}

<!-- ✅ -->
{@render mySnippet()}
```

### Snippets with typed parameters

```svelte
<!-- ✅ Type the parameters -->
{#snippet row(item: Item)}
  <td>{item.name}</td>
{/snippet}

<!-- ❌ Untyped — loses type safety -->
{#snippet row(item)}
  <td>{item.name}</td>
{/snippet}
```
