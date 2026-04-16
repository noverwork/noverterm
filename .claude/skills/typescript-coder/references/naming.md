# Naming Conventions

Follow Google TypeScript Style for naming, with one project-specific override for filenames.

## Naming Table

| Type                | Convention   | Example           | Source           |
| ------------------- | ------------ | ----------------- | ---------------- |
| Files               | `kebab-case` | `user-service.ts` | Project override |
| Variables/constants | `camelCase`  | `maxRetryCount`   | Official         |
| Functions           | `camelCase`  | `getUserById`     | Official         |
| Classes/interfaces  | `PascalCase` | `UserService`     | Official         |

> Google style uses lowercase paths; this repo intentionally uses `kebab-case` for filenames.

## File Naming

```
✅ CORRECT (project convention)
user-service.ts
create-user-command.ts
get-user-query.ts
meeting.controller.ts
notification.service.ts

❌ WRONG
userService.ts
CreateUserCommand.ts
get_user_query.ts
meetingController.ts
```

## Function Naming

```typescript
// ✅ CORRECT
function getUserById(
  repository: UserRepository,
  id: string,
): Promise<User | null> {
  return repository.findOne({ id });
}

async function processMeeting(
  commandBus: CommandBus,
  meetingId: string,
): Promise<void> {
  await commandBus.execute(new ProcessMeetingCommand(meetingId));
}

// ❌ WRONG
function ProcessMeeting() {}
function get_user_by_id(id: string) {}
```

## Class Naming

```typescript
// ✅ CORRECT
export class UserService {}
export class CreateMeetingCommand {}
export class GetMeetingDetailQuery {}
export class AdminGuard {}

// ❌ WRONG
export class user_service {}
export class createMeetingCommand {}
export class get_meeting_detail_query {}
```
