# Widget Patterns

This file mixes **project UI architecture** with a few **official** Flutter style principles.

## Adaptive Widgets (Project)

Layout is determined by **host OS**, not window width, on native platforms. Use `deviceTypeOf(context)` from `core/layout/device_type.dart`.

| Host                    | Layout      | Notes                                 |
| ----------------------- | ----------- | ------------------------------------- |
| macOS / Windows / Linux | desktop     | Min window 960×640                    |
| iOS / Android           | mobile      | iPad intentionally uses mobile layout |
| Web                     | width-based | `< 600` → mobile, `>= 600` → desktop  |

### Naming

- `Adaptive` prefix — wrapper that selects platform widget
- `Mobile` / `Desktop` prefix — platform-specific UI
- Small differences → single widget + `LayoutBuilder`
- Structural differences → separate widgets

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

## Per-Page Scaffold (Project)

Use a separate mobile `Scaffold` per branch page to preserve independent scroll/controller behavior inside `StatefulShellRoute.indexedStack`.

- Adaptive wrapper `Scaffold` wraps **only** the mobile variant
- New branch pages should own their mobile `Scaffold`
- Scroll views should use `PrimaryScrollController.of(context)` or no explicit controller

## Page Headers (Project)

Use shared header components instead of inline header rows.

```dart
DesktopPageHeader(
  title: AppLocalizations.of(context)!.sessionsTitle,
)

SearchableHeader(
  title: AppLocalizations.of(context)!.sessionsTitle,
)
```

## Hover Style (Project)

Desktop clickable elements should use `HoverBuilder`.

```dart
HoverBuilder(
  builder: (context, isHovered) {
    return GestureDetector(
      onTap: () {
        openDetails(context);
      },
      child: Container(
        color: isHovered
            ? Color.alphaBlend(hoverColor, baseColor)
            : baseColor,
      ),
    );
  },
)
```

- `HoverBuilder` wraps outside `GestureDetector`
- Blend overlays with `Color.alphaBlend`

## Design System (Project)

### Theme

- Material 3 (`useMaterial3: true`)
- Dark theme is primary; light theme is also supported
- Spacing scale: 4, 8, 12, 16, 24, 32, 48

### Colors

```dart
Widget build(BuildContext context) {
  return Text(
    AppLocalizations.of(context)!.greeting,
    style: TextStyle(color: context.colors.textPrimary),
  );
}
```

- Use `context.colors`, not static theme instances
- Add new colors to both dark and light factories

### Typography

- `AppTypography` is style-only; apply colors in the widget layer
- Fonts: DM Sans, Inter, Fraunces
