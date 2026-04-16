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
