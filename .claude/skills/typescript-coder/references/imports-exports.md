# Import & Export Rules

## Import Order

**Official:** place imports in sections and sort within sections.

```typescript
// 1. Side-effect imports
import 'reflect-metadata';

// 2. Namespace imports
import * as ng from '@angular/core';

// 3. Named/default imports
import dayjs from 'dayjs';
import { Injectable } from '@nestjs/common';

// 4. Relative imports
import { CreateUserDto } from './dto/create-user.dto';
import { User } from './models/user';
```

## Type-Only Imports

`import type` is allowed and often useful, but it is not a uniquely Google-only requirement.

```typescript
// ✅ GOOD
import type { Request, Response } from 'express';
import { type CommandBus, Injectable } from '@nestjs/common';

// ✅ ALSO OK
import { Request, Response } from 'express';
```

## Export Rules

**Official:** use named exports and avoid mutable exports.

```typescript
// ✅ GOOD
export class Foo {
  constructor(readonly value: number) {}
}

export const maxValue = 42;
export type UserId = string;

// ❌ BAD
export default class Foo {}

// ❌ BAD
export let counter = 0;

// ✅ GOOD
let counter = 0;
export function getCounter(): number {
  return counter;
}
```

## Modules, Not Namespaces

**Official:** use ES modules instead of TypeScript namespaces.

```typescript
// ❌ BAD
namespace UserHelpers {
  export function formatName(name: string): string {
    return name.trim();
  }
}

// ✅ GOOD
export function formatName(name: string): string {
  return name.trim();
}
```

## `const` / `let`

**Official:** use `const` and `let`, never `var`.

```typescript
// ✅ GOOD
const limit = 10;
let count = 0;

// ❌ BAD
var count = 0;
```
