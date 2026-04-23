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

## Arrow Functions as Properties

**Official:** classes usually should **not** contain properties initialized to arrow functions. Arrow function properties require the caller to understand that the callee's `this` is already bound, which increases confusion about what `this` is. Call sites using such handlers look broken (require non-local knowledge to determine correctness).

```typescript
// ❌ BAD - arrow function property
class MyComponent {
  private handler = () => {
    this.doSomething();
  };

  attach() {
    // Looks broken, but is actually correct (non-local knowledge needed)
    window.addEventListener('click', this.handler);
  }
}

// ✅ GOOD - use arrow function at call site to invoke instance method
class MyComponent {
  private doSomething(): void { ... }

  attach() {
    const handler = (e: Event) => { this.doSomething(); };
    window.addEventListener('click', handler);
  }
}
```

**Exception:** arrow function properties are the right approach when the handler requires uninstallation (e.g., event listeners), because they automatically capture `this` and provide a stable reference.

```typescript
// ✅ OK - stable reference needed for uninstallation
class MyComponent {
  private listener = () => {
    confirm('Do you want to exit the page?');
  };

  onAttached() {
    window.addEventListener('beforeunload', this.listener);
  }

  onDetached() {
    window.removeEventListener('beforeunload', this.listener);
  }
}
```

Code should **always** use arrow functions to call instance methods, and **should not** obtain or pass references to instance methods:

```typescript
// ✅ GOOD - arrow function wrapping the method call
const handler = (x: Event) => { this.listener(x); };

// ❌ BAD - passing reference to instance method directly
const handler = this.listener;
handler(x);
```
