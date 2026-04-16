# Anti-Patterns

Each item is labeled as **Official** or **Project**.

## 1. `Model` Suffix on Classes (Project)

```dart
// ❌ ANTI-PATTERN
class SessionModel {
  const SessionModel();
}

// ✅ CORRECT
class Session {
  const Session();
}
```

## 2. Direct Service Calls in Widgets (Project)

```dart
// ❌ ANTI-PATTERN
class BadSessionsPage extends StatelessWidget {
  const BadSessionsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final data = EdgeService().getSessions();
    return Text('$data');
  }
}

// ✅ CORRECT
class GoodSessionsPage extends ConsumerWidget {
  const GoodSessionsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final data = ref.watch(sessionListProvider);
    return data.when(
      data: (sessions) => SessionListView(sessions: sessions),
      loading: () => const CircularProgressIndicator(),
      error: (error, _) => Text(error.toString()),
    );
  }
}
```

## 3. Inline Page Header Rows (Project)

```dart
// ❌ ANTI-PATTERN
Row(
  children: [
    Text(AppLocalizations.of(context)!.sessionsTitle),
    const Spacer(),
    IconButton(onPressed: () {}, icon: const Icon(Icons.search)),
  ],
)

// ✅ CORRECT
DesktopPageHeader(title: AppLocalizations.of(context)!.sessionsTitle)
```

## 4. Hardcoded Strings (Project)

```dart
// ❌ ANTI-PATTERN
const Text('No sessions found');

// ✅ CORRECT
Text(AppLocalizations.of(context)!.noSessionsFound)
```

## 5. Direct Color Access (Project)

```dart
// ❌ ANTI-PATTERN
Text(
  AppLocalizations.of(context)!.greeting,
  style: TextStyle(color: AppColorsTheme.dark.textPrimary),
)

// ✅ CORRECT
Text(
  AppLocalizations.of(context)!.greeting,
  style: TextStyle(color: context.colors.textPrimary),
)
```

## 6. Hover Without HoverBuilder (Project)

```dart
// ❌ ANTI-PATTERN
GestureDetector(onTap: () {}, child: const Card())

// ✅ CORRECT
HoverBuilder(
  builder: (context, isHovered) {
    return GestureDetector(
      onTap: () {},
      child: const Card(),
    );
  },
)
```

## 7. `.length == 0` (Official)

```dart
// ❌ ANTI-PATTERN
if (items.length == 0) {
  return;
}

// ✅ CORRECT
if (items.isEmpty) {
  return;
}
```

## 8. `forEach` With Function Literal (Official)

```dart
// ❌ ANTI-PATTERN
items.forEach((item) {
  process(item);
});

// ✅ CORRECT
for (final item in items) {
  process(item);
}
```

## 9. `is` Type Checks on Children (Project, inspired by Flutter repo style)

```dart
// ❌ ANTI-PATTERN
Widget build(BuildContext context) {
  if (widget.child is Text) {
    return const SizedBox.shrink();
  }
  return widget.child;
}

// ✅ CORRECT
Widget build(BuildContext context) {
  return Container(child: widget.child);
}
```

## 10. Expensive Getters (Official, Flutter-style guidance)

```dart
// ❌ ANTI-PATTERN
List<Element> get allDescendants {
  final result = <Element>[];
  void walk(Element element) {
    result.add(element);
  }

  visitChildren(walk);
  return result;
}

// ✅ CORRECT
List<Element> findAllDescendants() {
  final result = <Element>[];
  visitChildren(result.add);
  return result;
}
```

## 11. `print()` in App Code (Project)

```dart
// ❌ ANTI-PATTERN
print('Recording started');

// ✅ CORRECT
if (kDebugMode) {
  debugPrint('Recording started');
}
```

## 12. Commented-Out Code (Common)

```dart
// ❌ ANTI-PATTERN
// final oldResult = computeLegacy(data);

// ✅ CORRECT
final result = computeNew(data);
```
