# Architecture

This file is **project-specific**. It is not part of Effective Dart.

## Platform vs Screen Size

Native layout is determined by host OS, not resized window width. Web uses width-based selection.

```dart
DeviceType deviceTypeOf(BuildContext context) {
  if (kIsWeb) {
    final width = MediaQuery.sizeOf(context).width;
    return width >= 600 ? DeviceType.desktop : DeviceType.mobile;
  }

  if (Platform.isIOS || Platform.isAndroid) {
    return DeviceType.mobile;
  }

  return DeviceType.desktop;
}
```

## Platform-Specific Services

Use abstract interfaces plus platform-aware factories.

```dart
abstract class FileStorage {
  Future<File> save(String name, Uint8List data);
}

class MobileFileStorage implements FileStorage {
  @override
  Future<File> save(String name, Uint8List data) async {
    throw UnimplementedError();
  }
}

class DesktopFileStorage implements FileStorage {
  @override
  Future<File> save(String name, Uint8List data) async {
    throw UnimplementedError();
  }
}

FileStorage createFileStorage() {
  if (kIsWeb) return WebFileStorage();
  if (Platform.isIOS || Platform.isAndroid) return MobileFileStorage();
  return DesktopFileStorage();
}
```

## Navigation (Project)

- Mobile uses a 4-tab floating pill navigation
- Desktop uses a left sidebar shell
- Mobile and desktop destinations should not be shared blindly
- Recording/detail/summary/transcription are push routes

## Directory Structure (Project)

Organize by feature, not by platform.

```text
lib/
├── core/
├── features/
├── models/
├── services/
└── shared/
```
