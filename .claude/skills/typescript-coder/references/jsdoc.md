# JSDoc & Documentation

## JSDoc vs Comments

**Official:** there are two types of comments:

- Use `/** JSDoc */` for **documentation** — comments a user of the code should read.
- Use `// line comments` for **implementation comments** — comments that only concern the implementation.

JSDoc comments are understood by tools (editors, documentation generators). Ordinary comments are only for other humans.

```typescript
// ✅ GOOD - JSDoc for documentation
/**
 * Finds a user by their email address.
 * Returns null if no user exists with the given email.
 */
function findUserByEmail(email: string): Promise<User | null> { ... }

// ✅ GOOD - line comment for implementation detail
function findUserByEmail(email: string): Promise<User | null> {
  // Normalize email to lowercase before querying
  const normalized = email.toLowerCase();
  return db.users.findOne({ email: normalized });
}

// ❌ BAD - JSDoc for implementation detail
/**
 * Normalize email to lowercase before querying
 */
const normalized = email.toLowerCase();
```

## Document All Top-Level Exports

**Official:** use `/** JSDoc */` comments to communicate information to users of your code. Document all exported functions, classes, and interfaces whose purpose is not immediately obvious.

```typescript
// ✅ GOOD
/**
 * Command bus for dispatching domain commands.
 * Handles command validation, middleware execution, and dispatch to handlers.
 */
export class CommandBus {
  /**
   * Execute a command and return the result.
   * Throws CommandHandlerNotFoundError if no handler is registered.
   */
  async execute<T, R>(command: Command<T>): Promise<R> { ... }
}
```

## Omit Comments Redundant with TypeScript

**Official:** do not declare types in `@param` or `@return` blocks. Do not write `@implements`, `@enum`, `@private`, `@override` on code that uses the corresponding TypeScript keywords.

```typescript
// ✅ GOOD - TypeScript already provides types
/**
 * Creates a new user account.
 * @param name The display name for the account.
 * @param email The primary email address.
 * @returns The created user with generated ID.
 */
function createUser(name: string, email: string): Promise<User> { ... }

// ❌ BAD - redundant type declarations
/**
 * @param name {string} The display name
 * @param email {string} The primary email
 * @returns {Promise<User>} The created user
 */
function createUser(name: string, email: string): Promise<User> { ... }

// ❌ BAD - redundant with TypeScript keywords
/**
 * @implements {UserService}
 * @private
 * @override
 */
class PostgresUserService implements UserService { ... }
```

## Do Not Use `@override`

**Official:** do not use `@override` in TypeScript source code. It is not enforced by the compiler, which is surprising and leads to annotations and implementation going out of sync.

```typescript
// ✅ GOOD - TypeScript's `override` keyword
class PostgresUserService extends BaseService {
  override async save(user: User): Promise<void> { ... }
}

// ❌ BAD - JSDoc @override
/** @override */
class PostgresUserService extends BaseService { ... }
```

## Make Comments That Actually Add Information

**Official:** avoid comments that just restate the parameter name and type. `@param` and `@return` lines are only required when they add information, and may otherwise be omitted.

```typescript
// ❌ BAD - just restates the name
/** @param fooBarService The Bar service for the Foo application. */

// ✅ GOOD - adds meaningful context
/**
 * @param timeout Maximum time to wait before retrying, in milliseconds.
 *   Defaults to 3000. Values below 100 are clamped to 100.
 */
function retryWithBackoff(timeout?: number): Promise<void> { ... }
```

## Parameter Property Comments

**Official:** to document constructor parameter properties, use `@param` annotations. Editors display the description on constructor calls and property accesses.

```typescript
// ✅ GOOD
class BrewService {
  /**
   * @param percolator The percolator used for brewing.
   * @param beans The beans to brew.
   */
  constructor(
    private readonly percolator: Percolator,
    private readonly beans: CoffeeBean[],
  ) {}
}
```

## Ordinary Field Comments

Document ordinary fields whose purpose is not immediately obvious.

```typescript
// ✅ GOOD
class BrewService {
  /** The bean that will be used in the next call to brew(). */
  nextBean: CoffeeBean;

  constructor(initialBean: CoffeeBean) {
    this.nextBean = initialBean;
  }
}
```

## JSDoc Placement Relative to Decorators

**Official:** when a class, method, or property has both decorators and JSDoc, write the JSDoc **before** the decorator.

```typescript
// ✅ GOOD
/**
 * Handles user authentication requests.
 */
@Injectable()
export class AuthService { ... }

// ❌ BAD
@Injectable()
/**
 * Handles user authentication requests.
 */
export class AuthService { ... }
```

## Block Comment Style

**Official:** block comments are indented at the same level as surrounding code. For multi-line `/* ... */` comments, subsequent lines must start with `*` aligned with the `*` on the previous line.

```typescript
// ✅ GOOD
/*
 * This is
 * okay.
 */

// And so
// is this.

/* You can
   do this too. */

// ❌ BAD - don't draw boxes
// ============================
// ||  IMPORTANT SECTION   ||
// ============================
```

Do not use JSDoc (`/** ... */`) for implementation comments.
