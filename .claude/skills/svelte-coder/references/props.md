# $props() & $bindable()

## $props() Basic Usage

```svelte
<script lang="ts">
  interface Props {
    name: string;
    count?: number;
    onChange: (value: number) => void;
  }

  // Destructuring with defaults
  let { name, count = 0, onChange }: Props = $props();
</script>
```

## Rest Props

```svelte
<script lang="ts">
  // Spread remaining props
  let { known, ...rest } = $props();
</script>

<button {...rest}>Click</button>
```

## $bindable() — Two-Way Binding

```svelte
<!-- FancyInput.svelte -->
<script lang="ts">
  interface Props {
    value: string;
    onChange?: (value: string) => void;
  }

  let { value = $bindable(''), onChange }: Props = $props();
</script>

<input bind:value={value} oninput={() => onChange?.(value)} />

<!-- Parent usage -->
<script lang="ts">
  import FancyInput from './FancyInput.svelte';
  let message = $state('hello');
</script>

<!-- With bind: — two-way binding -->
<FancyInput bind:value={message} />

<!-- Without bind: — one-way, onChange callback -->
<FancyInput value={message} onChange={(v) => message = v} />
```

## Controlled vs Uncontrolled Pattern

```svelte
<script lang="ts">
  interface Props {
    value?: string;
    defaultValue?: string;
    onChange?: (value: string) => void;
  }

  let { value, defaultValue = '', onChange }: Props = $props();

  // Controlled: parent manages state
  // Uncontrolled: component manages its own state
  let internalValue = $state(defaultValue);
  let isControlled = $derived(value !== undefined);

  function handleInput(e: Event) {
    const newValue = (e.target as HTMLInputElement).value;
    if (isControlled) {
      onChange?.(newValue);
    } else {
      internalValue = newValue;
    }
  }
</script>

<input value={isControlled ? value : internalValue} oninput={handleInput} />
```

## Optional Props

```svelte
<script lang="ts">
  interface Props {
    // Required — TypeScript errors if not provided
    title: string;

    // Optional with default
    count?: number;

    // Optional callback
    onSubmit?: () => void;

    // Optional snippet
    actions?: Snippet;
  }

  let { title, count = 0, onSubmit, actions }: Props = $props();
</script>
```

## Pitfalls

### Mutating non-bindable props

```svelte
<script lang="ts">
  let { value } = $props();

  // ⚠️ Svelte warns in dev mode — don't mutate props directly
  function mutate() {
    value = 'new value'; // Don't do this!
  }
</script>
```

### $bindable without bind: in parent

```svelte
<!-- Parent.svelte -->
<script lang="ts">
  let val = $state('hello');
</script>

<!-- Without bind: — changes in child won't propagate to parent -->
<Child value={val} />

<!-- With bind: — two-way binding works -->
<Child bind:value={val} />
```

### Can't tell if prop is bound at runtime

There's no runtime way to know if a parent used `bind:` or just passed the prop. Use callback props for controlled components:

```svelte
<script lang="ts">
  let { value, onChange } = $props();

  function handleInput(e: Event) {
    const newValue = (e.target as HTMLInputElement).value;
    onChange?.(newValue); // Always notify parent
  }
</script>

<input value={value} oninput={handleInput} />
```
