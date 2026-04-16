# Null Handling

## Optional Chaining

```typescript
// ✅ GOOD - Optional chaining + nullish coalescing
const userName = user?.profile?.name ?? 'Anonymous';
const userId = user?.id ?? 'guest';

// ❌ BAD - Verbose null checks
const userName =
  user && user.profile && user.profile.name ? user.profile.name : 'Anonymous';
```

## Non-Null Assertion

```typescript
// ⚠️ USE SPARINGLY - Only when you're certain it's not null
const element = document.getElementById('app')!; // Risky!

// ✅ BETTER - Explicit check
const element = document.getElementById('app');
if (!element) {
  throw new Error('App element not found');
}
```

## Type Guards for Null

```typescript
// ✅ GOOD - Type guard
function isNotNull<T>(value: T | null): value is T {
  return value !== null;
}

// Usage
if (isNotNull(user)) {
  // TypeScript knows user is not null here
  console.log(user.email);
}
```
