# Naming Conventions

Follow Effective Dart for naming. Project-specific Riverpod conventions are marked separately.

## Naming Table

| Type                | Convention             | Example                           | Source   |
| ------------------- | ---------------------- | --------------------------------- | -------- |
| Files/dirs          | `snake_case`           | `recording_page.dart`             | Official |
| Classes/types/enums | `PascalCase`           | `RecordingPage`, `Session`        | Official |
| Variables/functions | `camelCase`            | `startRecording()`, `sessionList` | Official |
| Constants           | `camelCase`            | `defaultPort`, `maxSessions`      | Official |
| Private members     | `_` prefix             | `_controller`, `_handleTap()`     | Official |
| Providers           | `camelCase + Provider` | `recordingStateProvider`          | Project  |

## File Naming

```dart
// ✅ CORRECT
recording_page.dart
edge_client_service.dart
session.dart

// ❌ WRONG
recordingPage.dart
EdgeClientService.dart
session_model.dart
```

## Class and Provider Naming

```dart
// ✅ CORRECT
class RecordingPage extends StatelessWidget {
  const RecordingPage({super.key});

  @override
  Widget build(BuildContext context) {
    return const SizedBox.shrink();
  }
}

final recordingStateProvider = StateProvider<RecordingState>((ref) {
  return RecordingState.idle;
});

// ❌ WRONG
class RecordingPageModel extends StatelessWidget {
  const RecordingPageModel({super.key});

  @override
  Widget build(BuildContext context) => const SizedBox.shrink();
}
```

## Key Effective Dart Rules

- **DO** use `lowercase_with_underscores` for libraries and packages
- **DO** use `PascalCase` for types
- **DO** use `camelCase` for variables, functions, parameters, and constants
- **DO** use `_` for library-private declarations
- **DON'T** add unnecessary type-name suffixes when the domain name is already clear
- **DON'T** use obscure abbreviations unless they are standard (`id`, `url`, `http`)
