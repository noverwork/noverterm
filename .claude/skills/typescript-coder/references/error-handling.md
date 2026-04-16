# Error Handling

## Custom Error Classes

```typescript
// ✅ GOOD - Custom error class with proper name
export class UserNotFoundException extends Error {
  constructor(userId: string) {
    super(`User not found: ${userId}`);
    this.name = 'UserNotFoundException';
  }
}

export class PaymentFailedError extends Error {
  constructor(paymentId: string, reason: string) {
    super(`Payment failed: ${paymentId} - ${reason}`);
    this.name = 'PaymentFailedError';
  }
}
```

## Type-Safe Error Handling

```typescript
// ✅ GOOD - Type-safe error handling with instanceof
try {
  await processPayment(paymentId);
} catch (error) {
  if (error instanceof UserNotFoundException) {
    return null;
  }
  if (error instanceof PaymentFailedError) {
    throw error; // Re-throw known errors
  }
  throw error; // Re-throw unknown errors
}
```

## Error Discriminator Pattern

```typescript
// ✅ GOOD - Discriminator union for errors
type AppError =
  | {
      type: 'user-not-found';
      userId: string;
    }
  | {
      type: 'payment-failed';
      paymentId: string;
      reason: string;
    };

function handleError(error: AppError): void {
  switch (error.type) {
    case 'user-not-found':
      console.log('User not found:', error.userId);
      break;
    case 'payment-failed':
      console.log('Payment failed:', error.paymentId);
      break;
  }
}
```
