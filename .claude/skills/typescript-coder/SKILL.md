---
name: typescript-coder
description: TypeScript coding guide for this monorepo. Use when writing TypeScript code including types, interfaces, classes, functions, generics, imports, and exports. Combines Google TypeScript Style with clearly marked project-specific rules.
---

# TypeScript Coder - Convention Guide

TypeScript conventions for this monorepo.

- **Official** = from Google TypeScript Style Guide
- **Project** = local rule for this repo
- **Common** = broadly accepted TypeScript practice, but not a Google rule

## Forbidden Patterns

```typescript
// ❌ NEVER (Project)
function process(data: any): any {
  return data;
}

const now = new Date();
const id = uuid();

// ✅ ALWAYS (Project)
function process(data: unknown): Result {
  return toResult(data);
}

const now = dayjs();
const id = nanoid();
```

## Quick Reference

### Naming

| Type                | Convention   | Example           | Source           |
| ------------------- | ------------ | ----------------- | ---------------- |
| Files               | `kebab-case` | `user-service.ts` | Project override |
| Variables/constants | `camelCase`  | `maxRetryCount`   | Official         |
| Functions           | `camelCase`  | `getUserById`     | Official         |
| Classes/interfaces  | `PascalCase` | `UserService`     | Official         |

### Key Rules

1. **[Official]** No default exports — use named exports only
2. **[Official]** No `export let` — export immutable bindings or accessor functions
3. **[Official]** No `#private` fields — use TypeScript visibility modifiers
4. **[Official]** Omit `public` except allowed parameter-property cases
5. **[Official]** Do not use `this` in static methods — reference the class name directly
6. **[Official]** Prefer function declarations for named functions
7. **[Official]** Use block bodies for arrow callbacks when the return value is unused
8. **[Official]** Use `const`/`let`, never `var`
9. **[Official]** Use `readonly` for fields that are not reassigned after construction
10. **[Official]** Use modules, not namespaces
11. **[Project]** Use `kebab-case` filenames instead of Google’s lowercase-with-underscores
12. **[Common]** Prefer union types to enums for simple closed sets

### Type Guards

```typescript
function isUser(value: unknown): value is User {
  return typeof value === 'object' && value !== null && 'id' in value;
}
```

### Type Definitions

```typescript
interface User {
  id: string;
}

type UserRole = 'admin' | 'user';
type PartialUser = Partial<User>;

type Status = 'active' | 'inactive';
```

## Detailed References

For detailed rules with examples, see:

- **Naming conventions** (files, functions, classes, constants) → `references/naming.md`
- **Function definitions** (declarations, arrow bodies, destructuring) → `references/functions.md`
- **Import & export** (order, type-only imports, `export type`, export rules) → `references/imports-exports.md`
- **Class conventions** (visibility, `readonly`, parameter properties, static methods) → `references/classes.md`
- **Generics** (official naming allowances, constraints) → `references/generics.md`
- **Null handling** (project/common guidance) → `references/null-handling.md`
- **Error handling** (project/common guidance) → `references/error-handling.md`
- **Anti-patterns** (official + project anti-patterns, clearly labeled) → `references/anti-patterns.md`
