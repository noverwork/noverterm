# Anti-Patterns

Each item is labeled as **Official** or **Project**.

## 1. Using `any` (Project)

```typescript
// ❌ ANTI-PATTERN
function process(data: any): any {
  return data;
}

// ✅ CORRECT
function process(data: unknown): Result {
  return toResult(data);
}
```

## 2. `new Date()` (Project)

```typescript
// ❌ ANTI-PATTERN
const now = new Date();
const timestamp = Date.now();

// ✅ CORRECT
const now = dayjs();
const timestamp = dayjs().valueOf();
```

## 3. `uuid` (Project)

```typescript
// ❌ ANTI-PATTERN
import { v4 as uuid } from 'uuid';
const id = uuid();

// ✅ CORRECT
import { nanoid } from 'nanoid';
const id = nanoid();
```

## 4. `snake_case` in TypeScript (Official)

```typescript
// ❌ ANTI-PATTERN
const user_id = '123';
function get_user_by_id(id: string): void {}

// ✅ CORRECT
const userId = '123';
function getUserById(id: string): void {}
```

## 5. `var` Declarations (Official)

```typescript
// ❌ ANTI-PATTERN
var count = 0;

// ✅ CORRECT
let count = 0;
const limit = 10;
```

## 6. Default Exports (Official)

```typescript
// ❌ ANTI-PATTERN
export default class UserService {}

// ✅ CORRECT
export class UserService {}
```

## 7. Mutable Exports (Official)

```typescript
// ❌ ANTI-PATTERN
export let counter = 0;

// ✅ CORRECT
let counter = 0;
export function getCounter(): number {
  return counter;
}
```

## 8. Function Expressions When Arrow Functions Suffice (Official)

```typescript
// ❌ ANTI-PATTERN
bar(function () {
  doSomething();
});

// ✅ CORRECT
bar(() => {
  doSomething();
});
```

## 9. Leaked Arrow Returns (Official)

```typescript
// ❌ ANTI-PATTERN
myPromise.then((value) => console.log(value));

// ✅ CORRECT
myPromise.then((value) => {
  console.log(value);
});
```

## 10. `this` in Static Context (Official)

```typescript
// ❌ ANTI-PATTERN
class ShoeStore {
  private static storage = new Map<string, boolean>();

  static isAvailable(id: string): boolean {
    return this.storage.has(id);
  }
}

// ✅ CORRECT
class ShoeStore {
  private static storage = new Map<string, boolean>();

  static isAvailable(id: string): boolean {
    return ShoeStore.storage.has(id);
  }
}
```

## 11. Unnecessary `public` Modifier (Official)

```typescript
// ❌ ANTI-PATTERN
class Foo {
  public bar = new Bar();
}

// ✅ CORRECT
class Foo {
  bar = new Bar();
}
```

## 12. `#private` Fields (Official)

```typescript
// ❌ ANTI-PATTERN
class Clazz {
  #ident = 1;
}

// ✅ CORRECT
class Clazz {
  private ident = 1;
}
```

## 13. Missing `readonly` on Stable Fields (Official)

```typescript
// ❌ ANTI-PATTERN
class Foo {
  private userList: string[] = [];
}

// ✅ CORRECT
class Foo {
  private readonly userList: string[] = [];
}
```

## 14. Verbose Null Checks (Common)

```typescript
// ❌ ANTI-PATTERN
const userName =
  user && user.profile && user.profile.name ? user.profile.name : 'Anonymous';

// ✅ CORRECT
const userName = user?.profile?.name ?? 'Anonymous';
```

## 15. Type Alias for Object Shapes (Official)

```typescript
// ❌ ANTI-PATTERN
type User = {
  id: string;
  name: string;
};

// ✅ CORRECT
interface User {
  id: string;
  name: string;
}
```

## 16. Type Assertion on Object Literals (Official)

```typescript
// ❌ ANTI-PATTERN - silently ignores missing fields
const foo = { bar: 42 } as Foo;

// ✅ CORRECT - catches missing fields at compile time
const foo: Foo = { bar: 42 };
```

## 17. Nullable Type Aliases (Official)

```typescript
// ❌ ANTI-PATTERN - hides where null originates
type MaybeUser = User | null;

// ✅ CORRECT - add null at point of use
function findUser(id: string): User | null { ... }
```

## 18. `| undefined` Instead of Optional (Official)

```typescript
// ❌ ANTI-PATTERN
interface Config {
  timeout: number | undefined;
}

// ✅ CORRECT
interface Config {
  timeout?: number;
}
```

## 19. Arrow Function Properties on Classes (Official)

```typescript
// ❌ ANTI-PATTERN - requires non-local knowledge
class MyComponent {
  private handler = () => { this.doSomething(); };
}

// ✅ CORRECT - use arrow at call site
class MyComponent {
  private doSomething(): void { ... }
  attach() {
    const handler = (e: Event) => { this.doSomething(); };
  }
}
```

## 20. Passing Named Functions Directly to Higher-Order Functions (Official)

```typescript
// ❌ ANTI-PATTERN - may receive unexpected extra arguments
myArray.forEach(fn);

// ✅ CORRECT - explicitly forward parameters
myArray.forEach((item) => fn(item));
```

## 21. Unfiltered `for...in` on Objects (Official)

```typescript
// ❌ ANTI-PATTERN - includes prototype properties
for (const key in obj) {
  doWork(obj[key]);
}

// ✅ CORRECT
for (const key of Object.keys(obj)) {
  doWork(obj[key]);
}
```

## 22. Missing Braces on Control Flow (Official)

```typescript
// ❌ ANTI-PATTERN
if (x) doSomething(x);
for (let i = 0; i < x; i++) doSomething(i);

// ✅ CORRECT
if (x) {
  doSomething(x);
}
```

## 23. Throwing Non-Error Values (Official)

```typescript
// ❌ ANTI-PATTERN - no stack trace
throw 'error message';
Promise.reject('something went wrong');

// ✅ CORRECT
throw new Error('error message');
Promise.reject(new Error('something went wrong'));
```

## 24. Redundant JSDoc Types (Official)

```typescript
// ❌ ANTI-PATTERN - TypeScript already provides types
/**
 * @param name {string} The user name
 * @returns {Promise<User>} The created user
 */
function createUser(name: string): Promise<User> { ... }

// ✅ CORRECT
/**
 * @param name The display name for the account.
 * @returns The created user with generated ID.
 */
function createUser(name: string): Promise<User> { ... }
```

## 25. `@override` JSDoc Tag (Official)

```typescript
// ❌ ANTI-PATTERN - not enforced by compiler
/** @override */
class Child extends Parent { ... }

// ✅ CORRECT - use TypeScript keyword
class Child extends Parent {
  override someMethod(): void { ... }
}
```
