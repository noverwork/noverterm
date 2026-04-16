# Generics

## Generic Parameter Names

**Official:** both single uppercase letters and descriptive UpperCamelCase names are acceptable.

```typescript
// ✅ GOOD
interface Box<T> {
  value: T;
}

// ✅ GOOD
interface Repository<TEntity> {
  find(criteria: Partial<TEntity>): Promise<TEntity[]>;
  save(entity: TEntity): Promise<TEntity>;
}
```

Use descriptive names when they improve readability in domain-heavy APIs.

## Generic Constraints

```typescript
// ✅ GOOD
function getProperty<TObject, TKey extends keyof TObject>(
  obj: TObject,
  key: TKey,
): TObject[TKey] {
  return obj[key];
}

interface EntityWithId {
  id: string;
}

// ❌ BAD
function getProperty<T>(obj: T, key: string): any {
  return (obj as Record<string, unknown>)[key];
}
```
