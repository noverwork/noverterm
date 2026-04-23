# Function Definitions

## Named Functions

**Official:** prefer function declarations for named functions.

```typescript
// ✅ GOOD
async function findUserById(
  repository: UserRepository,
  userId: string,
): Promise<User | null> {
  return repository.findOne({ id: userId });
}

function calculateTotal(items: Item[]): number {
  return items.reduce((sum, item) => sum + item.price, 0);
}

// ❌ BAD
const findUserById = async (userId: string): Promise<User | null> => {
  return userRepository.findOne({ id: userId });
};
```

Arrow functions are still appropriate for callbacks and for values typed by a function interface.

```typescript
interface SearchFunction {
  (source: string, subString: string): boolean;
}

const fooSearch: SearchFunction = (source, subString) => {
  return source.includes(subString);
};
```

## Arrow Function Bodies

**Official:** use block bodies when the return value is unused.

```typescript
// ✅ GOOD
myPromise.then((value) => {
  console.log(value);
});

// ✅ GOOD
const longThings = myValues
  .filter((value) => value.length > 1000)
  .map((value) => String(value));

// ❌ BAD
myPromise.then((value) => console.log(value));

// ✅ GOOD
myPromise.then((value) => void console.log(value));
```

## Function Expressions

**Official:** avoid function expressions when an arrow function or declaration will do.

```typescript
// ❌ BAD
bar(function () {
  doSomething();
});

// ✅ GOOD
bar(() => {
  doSomething();
});
```

Exception: function expressions are still appropriate when code must dynamically bind `this`, or for generator functions.

## Return Types

**Project preference:** annotate exported functions, public methods, and non-trivial functions when it improves readability.

```typescript
// ✅ GOOD - project preference for exported/public APIs
export async function findUserById(
  repository: UserRepository,
  userId: string,
): Promise<User | null> {
  return repository.findOne({ id: userId });
}

// ✅ ALSO OK - inference is allowed by Google style
function increment(value: number) {
  return value + 1;
}
```

## Destructuring

### Object Destructuring

```typescript
// ✅ GOOD
const { query, body } = request;
const { value: notifications = [] } = body;

// ❌ BAD
const notifications = body.value || [];
```

### Array Destructuring

```typescript
// ✅ GOOD
const [a, b, c, ...rest] = generateResults();
const [, second, , fourth] = someArray;

// ❌ BAD
const first = items[0];
const second = items[1];
const rest = items.slice(2);
```

### Parameter Destructuring

**Official:** keep parameter destructuring shallow and put defaults on the left side.

```typescript
// ✅ GOOD
function updateUser({ id, email = '' }: { id: string; email?: string }): void {
  saveUser(id, email);
}

function destructured([a = 4, b = 2] = []): void {
  usePair(a, b);
}

// ❌ BAD
function nestedTooDeeply({ x: { num, str } }: { x: Options }): void {}

// ❌ BAD
function badDestructuring([a, b] = [4, 2]): void {
  usePair(a, b);
}
```

## Prefer Passing Arrow Functions as Callbacks

**Official:** prefer passing arrow functions that explicitly forward parameters to named callbacks. Avoid passing a named function directly to a higher-order function unless you are sure of both functions' call signatures. Beware, in particular, of less-commonly-used optional parameters.

```typescript
// ❌ BAD - fn might receive unexpected extra arguments (index, array)
// This type-checks but can cause logical errors
myArray.forEach(fn);

// ✅ GOOD - arrow function explicitly forwards only the parameters fn expects
myArray.forEach((item) => fn(item));

// ✅ GOOD - explicit parameter forwarding
somePromise.then((value) => handleValue(value));
```

**Why:** higher-order functions may pass extra arguments (e.g., `forEach` passes `(item, index, array)`) that the callback doesn't expect. An arrow function wrapper ensures only the intended arguments are forwarded.
