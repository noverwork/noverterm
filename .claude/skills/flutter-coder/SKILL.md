---
name: flutter-coder
description: Flutter and Dart coding guide for this monorepo. Use when writing Flutter widgets, Riverpod providers, data models, routing, adaptive layouts, platform services, or i18n. Combines Effective Dart with clearly marked project-specific app conventions.
---

# Flutter Coder - Convention Guide

Flutter and Dart conventions for this monorepo.

- **Official** = Effective Dart or the Flutter repo style guide
- **Project** = local architecture/design rule for this app
- **Common** = common Flutter practice, not an explicit official rule

## Forbidden Patterns

```dart
// ❌ NEVER (Project)
print('debug');
Text('Hardcoded string');

// ❌ AVOID (Official/Common)
if (list.length == 0) {
  return;
}

items.forEach((item) {
  process(item);
});
```

## Quick Reference

### Naming

| Type                          | Convention   | Example                                      | Source             |
| ----------------------------- | ------------ | -------------------------------------------- | ------------------ |
| Files                         | `snake_case` | `recording_page.dart`                        | Official           |
| Classes/types                 | `PascalCase` | `RecordingPage`, `Session`                   | Official           |
| Variables/functions/providers | `camelCase`  | `startRecording()`, `recordingStateProvider` | Official + project |
| Constants                     | `camelCase`  | `defaultPort`                                | Official           |
| Private members               | `_` prefix   | `_controller`                                | Official           |

### Key Rules

1. **[Official]** Use `snake_case` for libraries/files
2. **[Official]** Use `.isEmpty` / `.isNotEmpty`, not `.length == 0`
3. **[Official]** Prefer `for` loops to `forEach` with a function literal
4. **[Official]** Do not use `new`
5. **[Official]** Do not use redundant `const` inside an already-const context
6. **[Official]** Avoid `this.` except for redirects or shadowing
7. **[Official]** Use braces for all flow control statements
8. **[Official]** Order imports as `dart:`, then `package:`, then relative; sort within sections
9. **[Official]** Use `dart format`
10. **[Project]** No direct service calls in widgets — read state through Riverpod providers
11. **[Project]** No hardcoded user-facing strings — use l10n
12. **[Project]** Layout follows host OS on native platforms; web uses width-based switching

### Widget Pattern

```dart
class AdaptiveFoo extends StatelessWidget {
  const AdaptiveFoo({super.key});

  @override
  Widget build(BuildContext context) {
    return deviceTypeOf(context) == DeviceType.desktop
        ? const DesktopFoo()
        : const MobileFoo();
  }
}
```

### State Management

```dart
final sessionListProvider = FutureProvider<List<Session>>((ref) async {
  final repository = ref.watch(sessionRepositoryProvider);
  return repository.fetchSessions();
});

Widget build(BuildContext context, WidgetRef ref) {
  final sessions = ref.watch(sessionListProvider);
  return sessions.when(
    data: (list) => SessionListView(sessions: list),
    loading: () => const CircularProgressIndicator(),
    error: (error, _) => Text(error.toString()),
  );
}
```

### Data Model

```dart
@freezed
class Session with _$Session {
  const factory Session({
    required String id,
    required String title,
    DateTime? createdAt,
  }) = _Session;

  factory Session.fromJson(Map<String, Object?> json) =>
      _$SessionFromJson(json);
}
```

## Detailed References

- **Naming conventions** (files, classes, providers, constants) → `references/naming.md`
- **Widget patterns** (adaptive layout, hover, headers, design system) → `references/widgets.md`
- **State management & data** (Riverpod, freezed, JSON, i18n) → `references/state-management.md`
- **Architecture** (platform detection, services, navigation, routing) → `references/architecture.md`
- **Documentation** (dartdoc, error messages, breadcrumbs) → `references/documentation.md`
- **Anti-patterns** (official + project anti-patterns, clearly labeled) → `references/anti-patterns.md`
