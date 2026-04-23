# Control Flow

## Braced Blocks

**Official:** control flow statements (`if`, `else`, `for`, `do`, `while`, etc.) always use braced blocks, even if the body contains only a single statement. The first statement of a non-empty block must begin on its own line.

```typescript
// ✅ GOOD
for (let i = 0; i < x; i++) {
  doSomething(i);
}

if (x) {
  doSomethingWithALongMethodNameThatForcesANewLine(x);
}

// ❌ BAD - missing braces
if (x) doSomething(x);
for (let i = 0; i < x; i++) doSomething(i);
```

**Exception:** single-line `if` statements where the entire statement fits on one line may elide the block.

```typescript
// ✅ OK - fits on one line
if (x) x.doFoo();
```

## Iterating Objects

**Official:** iterating objects with `for (... in ...)` is error prone — it includes enumerable properties from the prototype chain. Do not use unfiltered `for (... in ...)` statements.

```typescript
// ❌ BAD - x could come from a parent prototype!
for (const x in someObj) {
  // unsafe
}

// ✅ GOOD - filter with hasOwnProperty
for (const x in someObj) {
  if (!someObj.hasOwnProperty(x)) continue;
  // now x was definitely defined on someObj
}

// ✅ GOOD - prefer for-of with Object.keys/values/entries
for (const x of Object.keys(someObj)) {
  // safe
}
for (const [key, value] of Object.entries(someObj)) {
  // safe
}
```

## Iterating Arrays

**Official:** prefer `for (... of someArr)` to iterate over arrays. `Array.prototype.forEach` and vanilla `for` loops are also allowed.

```typescript
// ✅ GOOD - preferred
for (const x of someArr) {
  doWork(x);
}

// ✅ ALSO OK - index needed
for (let i = 0; i < someArr.length; i++) {
  const x = someArr[i];
  doWork(x, i);
}

// ✅ ALSO OK - entries for index + value
for (const [i, x] of someArr.entries()) {
  doWork(x, i);
}

// ✅ ALSO OK - forEach (but harder to debug and defeats some compiler checks)
someArr.forEach((x) => doWork(x));
```

## Switch Statements

**Official:** every `switch` statement must include a `default` group, even if it contains no code. Fall-through is allowed only with an explicit `// falls through` comment.

```typescript
// ✅ GOOD
switch (status) {
  case 'active':
    handleActive();
    break;
  case 'pending':
    handlePending();
    // falls through
  case 'inactive':
    handleInactive();
    break;
  default:
    // exhaustive check
    assertNever(status);
}
```
