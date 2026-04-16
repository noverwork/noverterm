# Class Conventions

## Visibility

**Official:** omit `public` because it is the default.

```typescript
// ✅ GOOD
class Foo {
  bar = new Bar();
  doThing(): void {}
}

// ❌ BAD
class Foo {
  public bar = new Bar();
  public doThing(): void {}
}
```

Allowed exception:

```typescript
class Foo {
  constructor(public baz: Baz) {}
}
```

## Private Fields

**Official:** prefer TypeScript visibility modifiers over `#private` fields.

```typescript
// ❌ BAD
class Clazz {
  #ident = 1;
}

// ✅ GOOD
class Clazz {
  private ident = 1;
}
```

## Static Methods

**Official:** do not use `this` in static methods.

```typescript
// ❌ BAD
class ShoeStore {
  private static storage = new Map<string, boolean>();

  static isAvailable(id: string): boolean {
    return this.storage.has(id);
  }
}

// ✅ GOOD
class ShoeStore {
  private static storage = new Map<string, boolean>();

  static isAvailable(id: string): boolean {
    return ShoeStore.storage.has(id);
  }
}
```

## Module-Local Helpers

**Official:** prefer module-local helper functions to private static helpers when that keeps code simpler.

```typescript
function formatErrorMessage(code: string, message: string): string {
  return `[${code}] ${message}`;
}

export class ApiError extends Error {
  constructor(code: string, message: string) {
    super(formatErrorMessage(code, message));
    this.name = 'ApiError';
  }
}
```

## Parameter Properties

**Official:** parameter properties are fine when they reduce boilerplate.

```typescript
// ✅ GOOD
class Foo {
  constructor(private readonly barService: BarService) {}
}

// ✅ ALSO OK
class Bar {
  private readonly barService: BarService;

  constructor(barService: BarService) {
    this.barService = barService;
  }
}
```

## `readonly`

**Official:** use `readonly` for fields that are not reassigned after initialization.

```typescript
// ✅ GOOD
class Foo {
  private readonly userList: string[] = [];
}

// ❌ BAD
class Foo {
  private userList: string[] = [];
}
```
