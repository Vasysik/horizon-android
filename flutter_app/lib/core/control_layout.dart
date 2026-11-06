import 'dart:convert';
import 'dart:io';

class TouchPlacement {
  TouchPlacement({
    required this.x,
    required this.y,
    this.size = 0.1,
    this.opacity = 0.82,
  });

  double x;
  double y;
  double size;
  double opacity;

  Map<String, dynamic> toJson() {
    return {
      'center': [x, y],
      'size': size,
      'opacity': opacity,
    };
  }

  factory TouchPlacement.fromJson(Object? value) {
    if (value is! Map) {
      return TouchPlacement(x: 0.5, y: 0.5);
    }
    final center = value['center'];
    final x = center is List && center.isNotEmpty && center[0] is num
        ? (center[0] as num).toDouble()
        : 0.5;
    final y = center is List && center.length > 1 && center[1] is num
        ? (center[1] as num).toDouble()
        : 0.5;
    return TouchPlacement(
      x: x.clamp(0.025, 0.975).toDouble(),
      y: y.clamp(0.025, 0.975).toDouble(),
      size: value['size'] is num
          ? (value['size'] as num).toDouble().clamp(0.045, 0.45).toDouble()
          : 0.1,
      opacity: value['opacity'] is num
          ? (value['opacity'] as num).toDouble().clamp(0.0, 1.0).toDouble()
          : 0.82,
    );
  }

  TouchPlacement copy() {
    return TouchPlacement(x: x, y: y, size: size, opacity: opacity);
  }
}

class TouchButton {
  TouchButton({
    required this.id,
    required this.keys,
    required this.placement,
    this.label,
  });

  final String id;
  List<String> keys;
  TouchPlacement placement;

  /// null derives the label from the keys; an empty string hides it.
  String? label;

  String get displayLabel {
    if (label != null) {
      return label!;
    }
    if (keys.isEmpty) {
      return '';
    }
    return keys.map(touchKeyLabel).join(' + ');
  }
}

class TouchLayout {
  TouchLayout({required this.joystick, required this.buttons});

  final TouchPlacement joystick;
  final List<TouchButton> buttons;

  static const int controlCount = 14;

  static TouchLayout defaults() {
    return TouchLayout(
      joystick: TouchPlacement(x: 0.15, y: 0.72, size: 0.30, opacity: 0.9),
      buttons: [
        TouchButton(
          id: 'jump',
          keys: const ['action:key.jump'],
          placement: TouchPlacement(x: 0.90, y: 0.70, size: 0.105),
        ),
        TouchButton(
          id: 'attack',
          keys: const ['action:key.attack'],
          placement: TouchPlacement(x: 0.76, y: 0.58, size: 0.11),
        ),
        TouchButton(
          id: 'use',
          keys: const ['action:key.use'],
          placement: TouchPlacement(x: 0.90, y: 0.45, size: 0.10),
        ),
        TouchButton(
          id: 'inventory',
          keys: const ['action:key.inventory'],
          placement: TouchPlacement(x: 0.76, y: 0.84, size: 0.085),
        ),
        TouchButton(
          id: 'sneak',
          keys: const ['action:key.sneak'],
          placement: TouchPlacement(x: 0.62, y: 0.88, size: 0.085),
        ),
        TouchButton(
          id: 'pause',
          keys: const ['key.keyboard.escape'],
          placement: TouchPlacement(x: 0.94, y: 0.12, size: 0.075),
        ),
      ],
    );
  }

  static Future<TouchLayout> load(String directory) async {
    final file = File('$directory/horizon-touch.json');
    if (!await file.exists()) {
      return defaults();
    }
    try {
      final decoded = jsonDecode(await file.readAsString());
      if (decoded is! Map) {
        return defaults();
      }
      final value = Map<String, dynamic>.from(decoded);
      final layouts = value['layouts'];
      final first = layouts is List && layouts.isNotEmpty && layouts[0] is List
          ? layouts[0] as List
          : null;
      final joystick = first != null && first.isNotEmpty
          ? TouchPlacement.fromJson(first[0])
          : defaults().joystick;

      final rawButtons = value['custom_buttons'];
      if (rawButtons is! List) {
        return defaults();
      }
      final buttons = <TouchButton>[];
      for (final item in rawButtons) {
        if (item is! Map) {
          continue;
        }
        final button = Map<String, dynamic>.from(item);
        final keys = <String>[];
        final rawKeys = button['keys'];
        if (rawKeys is List) {
          for (final key in rawKeys) {
            if (key is String && key.isNotEmpty) {
              keys.add(key);
            }
          }
        }
        if (keys.isEmpty && button['key'] is String) {
          keys.add(button['key'] as String);
        }
        final rawLayouts = button['layouts'];
        final placement =
            rawLayouts is List && rawLayouts.isNotEmpty && rawLayouts[0] is Map
            ? TouchPlacement.fromJson(rawLayouts[0])
            : TouchPlacement(x: 0.7, y: 0.4, size: 0.1);
        final label = button['label'] is String
            ? button['label'] as String
            : null;
        buttons.add(
          TouchButton(
            id: 'b${buttons.length}',
            keys: keys,
            label: label,
            placement: placement,
          ),
        );
      }
      return TouchLayout(joystick: joystick, buttons: buttons);
    } catch (_) {
      return defaults();
    }
  }

  Map<String, dynamic> toGameJson() {
    final hidden = TouchPlacement(x: 0.5, y: 0.5, size: 0.1, opacity: 0.0);
    final row = List<Map<String, dynamic>>.generate(
      controlCount,
      (_) => hidden.toJson(),
    );
    row[0] = joystick.toJson();
    return {
      'mode': 'crosshair',
      'button_visibility': [1, for (var i = 1; i < controlCount; i++) 2],
      'layouts': [row, row, row],
      'custom_buttons': [
        for (final button in buttons)
          {
            'key': button.keys.isEmpty
                ? 'key.keyboard.unknown'
                : button.keys.first,
            'keys': button.keys,
            if (button.label != null) 'label': button.label,
            'layouts': [
              button.placement.toJson(),
              button.placement.toJson(),
              button.placement.toJson(),
            ],
          },
      ],
    };
  }

  Future<void> save(String directory) async {
    final folder = Directory(directory);
    if (!await folder.exists()) {
      await folder.create(recursive: true);
    }
    final file = File('$directory/horizon-touch.json');
    final partial = File('${file.path}.partial');
    await partial.writeAsString(jsonEncode(toGameJson()), flush: true);
    await partial.rename(file.path);
  }
}

String touchKeyLabel(String token) {
  if (token.startsWith('action:')) {
    const names = {
      'key.attack': 'Attack',
      'key.use': 'Use',
      'key.jump': 'Jump',
      'key.sneak': 'Sneak',
      'key.sprint': 'Sprint',
      'key.inventory': 'Inventory',
      'key.chat': 'Chat',
      'key.drop': 'Drop',
      'key.swapOffhand': 'Swap',
      'key.togglePerspective': 'Perspective',
    };
    return names[token.substring(7)] ?? token.substring(7);
  }
  final raw = token.replaceFirst('key.keyboard.', '');
  switch (raw) {
    case 'space':
      return 'Space';
    case 'left.shift':
      return 'Shift';
    case 'left.control':
      return 'Ctrl';
    case 'left.alt':
      return 'Alt';
    case 'escape':
      return 'Esc';
    case 'enter':
      return 'Enter';
    case 'tab':
      return 'Tab';
    case 'up':
      return 'Up';
    case 'down':
      return 'Down';
    case 'left':
      return 'Left';
    case 'right':
      return 'Right';
  }
  if (raw.length == 1) {
    return raw.toUpperCase();
  }
  return raw
      .split('.')
      .map(
        (part) => part.isEmpty
            ? part
            : '${part[0].toUpperCase()}${part.substring(1)}',
      )
      .join(' ');
}
