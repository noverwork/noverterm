# Documentation (Dartdoc)

Follow Effective Dart documentation guide, with Flutter repo conventions.

## Public Members

All public members in Flutter libraries should have documentation.

```dart
/// An object representing a recorded meeting session.
///
/// To create a [Session], use [Session.fromJson].
///
/// See also:
///
/// * [SessionList], which displays multiple sessions.
class Session {
  /// The unique identifier for this session.
  final String id;

  /// The title of the session.
  ///
  /// Changing the title will cause the session list to refresh.
  final String title;

  const Session({required this.id, required this.title});
}
```

## Key Rules

- First paragraph must be a short self-contained sentence (used in TOCs)
- Use `[square brackets]` for linking to Dart symbols
- Use `` `backticks` `` for constructor parameters that aren't also properties
- Avoid useless docs: "The background color." is useless for `backgroundColor`
- Each "See also:" list item ends with a period. Use "which..." rather than parentheticals
- Never use "you" or "we". Avoid imperative voice. Use "Consider" instead
- Never use "simply" or "just" — if they're reading docs, it's not easy

## Private Documentation

- Use `///` for public-quality private docs (good enough to make public verbatim)
- Use `//` for private docs that aren't publication quality
- This signals which private docs need review before making code public

## Error Messages

Every error message is an opportunity to make someone love the product:

```dart
// ✅ GOOD — actionable, explains why and how to fix
throw StateError(
  'SessionController.start() was called while already recording. '
  'Call stop() before starting a new recording.',
);

// ❌ BAD — vague
throw StateError('Invalid state');
```

## Breadcrumbs

Include usage hints in class docs:

```dart
/// A controller for managing recording state.
///
/// To obtain a controller, use [RecordingManager.createController].
/// The controller should be disposed when the widget is removed.
class RecordingController extends ChangeNotifier {
  void start() {
    notifyListeners();
  }
}
```
