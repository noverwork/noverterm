# State Management & Data

## Riverpod (Project)

- Use Riverpod + codegen (`riverpod_annotation`, `riverpod_generator`)
- Providers live by feature under `features/<name>/providers/`
- Async state should use `AsyncValue`
- Widgets read state from providers; they do not call services directly

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

// ❌ WRONG — direct service call in a widget
class BadSessionsPage extends StatelessWidget {
  const BadSessionsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final sessions = SessionService().getSessions();
    return Text('$sessions');
  }
}
```

## Data Models (Project + Common)

- Use `freezed` + `json_serializable`
- Use domain names directly (`Session`, not `SessionModel`)
- Commit generated `*.g.dart` and `*.freezed.dart`

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

## JSON (Project)

- API keys use `snake_case`
- Dart fields use `camelCase`
- Dates use ISO 8601
- Omit nulls where possible (`includeIfNull: false`)

## i18n (Project)

- All user-visible text goes through localization
- Locale files live in `lib/l10n/`
- Add both `zh` and `en` keys for new UI text

```dart
Text(AppLocalizations.of(context)!.noSessionsFound)
```
