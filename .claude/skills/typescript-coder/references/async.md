# Async & Promise Patterns

## Prefer `async/await` Over Raw Promises

**Common:** prefer `async/await` for readability and easier error handling. Raw `.then()`/`.catch()` chains are acceptable for simple cases but `async/await` should be the default.

```typescript
// ✅ GOOD - async/await
async function loadUser(userId: string): Promise<User> {
  const profile = await fetchProfile(userId);
  const settings = await fetchSettings(userId);
  return { ...profile, settings };
}

// ✅ ALSO OK - simple .then() chain
function loadConfig(): Promise<Config> {
  return fetch('/config').then((res) => res.json());
}

// ❌ AVOID - deeply nested .then() chains
function loadUser(userId: string): Promise<User> {
  return fetchProfile(userId)
    .then((profile) => {
      return fetchSettings(userId)
        .then((settings) => {
          return { ...profile, settings };
        });
    });
}
```

## Only Throw `Error` Objects

**Official:** only throw (subclasses of) `Error`. Throwing arbitrary values does not populate stack trace information, making debugging hard. This extends to `Promise` rejection values since `Promise.reject(obj)` is equivalent to `throw obj` in async functions.

```typescript
// ✅ GOOD - throw Error
throw new Error('oh noes!');

// ✅ GOOD - throw Error subclass
class MyError extends Error {}
throw new MyError('my oh noes!');

// ✅ GOOD - Promise rejection with Error
new Promise((resolve, reject) => void reject(new Error('oh noes!')));
Promise.reject(new Error('oh noes!'));

// ❌ BAD - no stack trace
throw 'oh noes!';
throw { message: 'error' };

// ❌ BAD - Promise rejection without Error
new Promise((resolve, reject) => void reject('oh noes!'));
Promise.reject();
Promise.reject('oh noes!');
```

## Error Handling in Async Functions

**Project preference:** use try/catch for async operations and re-throw with proper context. Do not swallow errors by returning `null` or empty values.

```typescript
// ✅ GOOD - proper error handling with context
async function loadResource(path: string): Promise<Resource> {
  try {
    const content = await fs.readFile(path, 'utf-8');
    return parseResource(content);
  } catch (error) {
    throw new ResourceLoadError(`Cannot load resource at ${path}`, { cause: error });
  }
}

// ❌ BAD - swallowing errors
async function loadResource(path: string): Promise<Resource | null> {
  try {
    return parseResource(await fs.readFile(path, 'utf-8'));
  } catch {
    return null;
  }
}
```

## Parallel Async Operations

Use `Promise.all` for independent async operations, `Promise.allSettled` when some failures are acceptable.

```typescript
// ✅ GOOD - parallel independent operations
async function loadDashboard(userId: string): Promise<Dashboard> {
  const [profile, notifications, stats] = await Promise.all([
    fetchProfile(userId),
    fetchNotifications(userId),
    fetchStats(userId),
  ]);
  return { profile, notifications, stats };
}

// ✅ GOOD - tolerate partial failures
async function sendNotifications(users: User[]): Promise<NotificationResult[]> {
  const results = await Promise.allSettled(
    users.map((u) => sendNotification(u)),
  );
  return results.map((r, i) => ({
    userId: users[i].id,
    success: r.status === 'fulfilled',
    error: r.status === 'rejected' ? r.reason : undefined,
  }));
}
```

## Promise Constructor

When creating new Promises, always resolve or reject. Prefer `async/await` over manual Promise construction.

```typescript
// ✅ GOOD - both resolve and reject paths
function delay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

// ✅ GOOD - explicit rejection with Error
function fetchWithTimeout(url: string, ms: number): Promise<Response> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Request timeout')), ms);
    fetch(url)
      .then((res) => {
        clearTimeout(timer);
        resolve(res);
      })
      .catch((err) => {
        clearTimeout(timer);
        reject(err);
      });
  });
}
```
