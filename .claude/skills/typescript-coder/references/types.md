# Type System

## Type Alias vs Interface

**Official:** use `type` aliases for primitives, unions, tuples, and mapped types. Use `interface` for object shapes.

```typescript
// ✅ GOOD - type alias for unions, primitives, tuples
type Status = 'active' | 'inactive';
type MyNumber = number;
type Point = [number, number];
type Result<T> = { ok: true; value: T } | { ok: false; error: string };

// ✅ GOOD - interface for object shapes
interface User {
  id: string;
  name: string;
}

// ❌ BAD - type alias for object literal
type User = {
  id: string;
  name: string;
};
```

**Why:** Interfaces create a new name used everywhere (better error messages), support `extends` with clear conflict errors, and support declaration merging. The TypeScript team lead recommends: "interfaces for anything that they can model."

## Explicit Type Annotations

**Official:** leave out type annotations for trivially inferred types (string, number, boolean, RegExp literal, or `new` expression). Add annotations when they improve clarity or prevent inference as `unknown`.

```typescript
// ✅ GOOD - no annotation needed, trivially inferred
const name = 'Alice';
const count = 42;
const isActive = true;
const pattern = /foo/;
const user = new User();

// ✅ GOOD - annotation required, prevents unknown inference
const items: User[] = [];
const cache = new Map<string, Config>();

// ✅ GOOD - annotation improves readability for complex returns
function getUsers(): Promise<User[]> {
  return db.query('SELECT * FROM users');
}

// ❌ BAD - redundant annotation
const name: string = 'Alice';
const count: number = 42;
```

**Rule of thumb:** Whether an annotation is required is decided by the code reviewer. When in doubt, annotate exported functions and public APIs.

## Type Assertions vs Type Annotations

**Official:** use type annotations (`: Foo`) instead of type assertions (`as Foo`) to specify the type of an object literal. This allows detecting refactoring bugs when interface fields change.

```typescript
interface Foo {
  bar: number;
  baz?: string;
}

// ✅ GOOD - type annotation catches missing fields at compile time
const foo: Foo = {
  bar: 42,
};

// ❌ BAD - type assertion silently ignores missing fields
const foo = {
  bar: 42,
} as Foo;
```

## Prefer Optional Over `| undefined`

**Official:** use optional fields (`?`) and parameters rather than `| undefined` union types.

```typescript
// ✅ GOOD - optional field
interface CoffeeOrder {
  sugarCubes: number;
  milk?: 'whole' | 'low-fat' | 'half-half';
}

// ✅ GOOD - optional parameter
function pourCoffee(volume?: number) { ... }

// ❌ BAD - explicit undefined union
interface CoffeeOrder {
  sugarCubes: number;
  milk: 'whole' | 'low-fat' | 'half-half' | undefined;
}
```

For classes, prefer initializing fields rather than making them optional:

```typescript
// ✅ GOOD - initialized field
class MyClass {
  field = '';
}

// ❌ AVOID - optional field without initialization
class MyClass {
  field?: string;
}
```

## Nullable/Undefined Type Aliases

**Official:** type aliases must not include `| null` or `| undefined` in a union type. Nullable aliases indicate that null values are being passed through too many layers, clouding the source of the original issue.

```typescript
// ❌ BAD - nullable type alias
type MaybeUser = User | null;

// ✅ GOOD - add null/undefined at the point of use
function findUser(id: string): User | null {
  return db.findById(id);
}
```

## `unknown` vs `any`

**Project preference:** use `unknown` instead of `any` for values of uncertain type. Always narrow with type guards before use.

```typescript
// ✅ GOOD - unknown requires narrowing
function parseInput(input: unknown): User {
  if (typeof input === 'object' && input !== null && 'id' in input) {
    return input as User;
  }
  throw new Error('Invalid input');
}

// ❌ BAD - any bypasses type checking
function parseInput(input: any): User {
  return input;
}
```

## Enums

**Common:** prefer union types to enums for simple closed sets. Enums add runtime overhead and have quirks (numeric enums are reversible, const enums have bundling issues).

```typescript
// ✅ GOOD - union type
type UserRole = 'admin' | 'user' | 'guest';
type Status = 'active' | 'inactive' | 'pending';

// ❌ AVOID - enum for simple values
enum UserRole {
  Admin = 'admin',
  User = 'user',
  Guest = 'guest',
}
```

Use `enum` only when you need:
- Numeric enums with auto-incrementing values
- Reverse mapping (enum value → name)
- Integration with external systems that expect enum objects

## Use Interfaces for Structural Types, Not Classes

**Official:** use interfaces to define structural types (object shapes), not classes. Classes should be concrete implementations.

```typescript
// ✅ GOOD - interface for structural type
interface UserRepository {
  findById(id: string): Promise<User | null>;
  save(user: User): Promise<void>;
}

// ✅ GOOD - class implements the interface
class PostgresUserRepository implements UserRepository {
  findById(id: string): Promise<User | null> { ... }
  save(user: User): Promise<void> { ... }
}
```
