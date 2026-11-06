import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core/app_state.dart';
import '../core/app_strings.dart';
import '../core/control_layout.dart';
import 'widgets/horizon_toast.dart';

class ControlEditorScreen extends StatefulWidget {
  const ControlEditorScreen({
    super.key,
    required this.state,
    required this.layout,
  });

  final AppState state;
  final TouchLayout layout;

  @override
  State<ControlEditorScreen> createState() => _ControlEditorScreenState();
}

class _ControlEditorScreenState extends State<ControlEditorScreen> {
  late TouchLayout _layout;
  late final TextEditingController _labelController;
  String _selected = 'joystick';
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _layout = TouchLayout(
      joystick: widget.layout.joystick.copy(),
      buttons: [
        for (final button in widget.layout.buttons)
          TouchButton(
            id: button.id,
            keys: List<String>.of(button.keys),
            label: button.label,
            placement: button.placement.copy(),
          ),
      ],
    );
    _labelController = TextEditingController();
    _syncLabel();
    SystemChrome.setPreferredOrientations(const [
      DeviceOrientation.landscapeLeft,
      DeviceOrientation.landscapeRight,
    ]);
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.immersiveSticky);
  }

  @override
  void dispose() {
    _labelController.dispose();
    SystemChrome.setPreferredOrientations(const [
      DeviceOrientation.portraitUp,
      DeviceOrientation.portraitDown,
    ]);
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
    super.dispose();
  }

  TouchButton? get _button {
    final index = _layout.buttons.indexWhere(
      (button) => button.id == _selected,
    );
    return index < 0 ? null : _layout.buttons[index];
  }

  void _syncLabel() {
    final button = _button;
    _labelController.text = button?.label ?? '';
  }

  void _select(String id) {
    if (_selected == id) {
      return;
    }
    setState(() {
      _selected = id;
      _syncLabel();
    });
  }

  Future<void> _addButton() async {
    final keys = await showDialog<List<String>>(
      context: context,
      builder: (_) => _KeyPickerDialog(
        title: context.strings.t('chooseKeys'),
        initial: const [],
      ),
    );
    if (keys == null || keys.isEmpty || !mounted) {
      return;
    }
    final index = _layout.buttons.length;
    final button = TouchButton(
      id: 'b${DateTime.now().microsecondsSinceEpoch}',
      keys: keys,
      label: context.strings.t('newButton'),
      placement: TouchPlacement(
        x: (0.58 + (index % 3) * 0.12).clamp(0.1, 0.94).toDouble(),
        y: (0.30 + (index ~/ 3 % 3) * 0.14).clamp(0.1, 0.9).toDouble(),
        size: 0.1,
      ),
    );
    setState(() {
      _layout.buttons.add(button);
      _selected = button.id;
      _syncLabel();
    });
  }

  Future<void> _editKeys() async {
    final button = _button;
    if (button == null) {
      return;
    }
    final keys = await showDialog<List<String>>(
      context: context,
      builder: (_) => _KeyPickerDialog(
        title: context.strings.t('chooseKeys'),
        initial: button.keys,
      ),
    );
    if (keys == null || !mounted) {
      return;
    }
    setState(() {
      button.keys = keys;
    });
  }

  void _deleteButton() {
    final button = _button;
    if (button == null) {
      return;
    }
    setState(() {
      _layout.buttons.removeWhere((item) => item.id == button.id);
      _selected = 'joystick';
      _syncLabel();
    });
  }

  Future<void> _reset() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) {
        return AlertDialog(
          title: Text(dialogContext.strings.t('resetLayout')),
          content: Text(dialogContext.strings.t('resetLayoutHint')),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(false),
              child: Text(dialogContext.strings.t('cancel')),
            ),
            FilledButton(
              onPressed: () => Navigator.of(dialogContext).pop(true),
              child: Text(dialogContext.strings.t('resetLayout')),
            ),
          ],
        );
      },
    );
    if (confirmed != true || !mounted) {
      return;
    }
    setState(() {
      _layout = TouchLayout.defaults();
      _selected = 'joystick';
      _syncLabel();
    });
  }

  Future<void> _save() async {
    setState(() => _saving = true);
    await widget.state.saveTouchLayout(_layout);
    if (!mounted) {
      return;
    }
    setState(() => _saving = false);
    showHorizonToast(context, context.strings.t('controlsSaved'));
    Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final strings = context.strings;
    return Scaffold(
      backgroundColor: const Color(0xFF0C110E),
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final panelWidth = constraints.maxWidth < 620 ? 280.0 : 340.0;
            return Row(
              children: [
                Expanded(
                  child: _Canvas(
                    layout: _layout,
                    selected: _selected,
                    onSelect: _select,
                    onChanged: () => setState(() {}),
                  ),
                ),
                SizedBox(
                  width: panelWidth,
                  child: Container(
                    decoration: BoxDecoration(
                      color: scheme.surfaceContainerLow,
                      border: Border(
                        left: BorderSide(
                          color: scheme.outlineVariant.withOpacity(0.55),
                        ),
                      ),
                    ),
                    child: Column(
                      children: [
                        Padding(
                          padding: const EdgeInsets.fromLTRB(14, 10, 8, 8),
                          child: Row(
                            children: [
                              IconButton(
                                tooltip: strings.t('close'),
                                onPressed: () => Navigator.of(context).pop(),
                                icon: const Icon(Icons.close_rounded),
                              ),
                              const SizedBox(width: 2),
                              Expanded(
                                child: Text(
                                  strings.t('controlEditorTitle'),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: Theme.of(context).textTheme.titleMedium
                                      ?.copyWith(fontWeight: FontWeight.w800),
                                ),
                              ),
                              IconButton(
                                tooltip: strings.t('resetLayout'),
                                onPressed: _reset,
                                icon: const Icon(Icons.restart_alt),
                              ),
                            ],
                          ),
                        ),
                        Padding(
                          padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
                          child: SizedBox(
                            width: double.infinity,
                            child: FilledButton.tonalIcon(
                              onPressed: _addButton,
                              icon: const Icon(Icons.add_rounded),
                              label: Text(strings.t('addButton')),
                            ),
                          ),
                        ),
                        Divider(
                          height: 1,
                          color: scheme.outlineVariant.withOpacity(0.4),
                        ),
                        Expanded(
                          child: SingleChildScrollView(
                            padding: const EdgeInsets.fromLTRB(16, 16, 16, 18),
                            child: _button == null
                                ? _joystickProperties(context)
                                : _buttonProperties(context, _button!),
                          ),
                        ),
                        Padding(
                          padding: const EdgeInsets.fromLTRB(16, 8, 16, 14),
                          child: SizedBox(
                            width: double.infinity,
                            child: FilledButton.icon(
                              onPressed: _saving ? null : _save,
                              icon: _saving
                                  ? const SizedBox(
                                      width: 18,
                                      height: 18,
                                      child: CircularProgressIndicator(
                                        strokeWidth: 2,
                                        color: Colors.white,
                                      ),
                                    )
                                  : const Icon(Icons.save_rounded, size: 20),
                              label: Text(strings.t('saveLayout')),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }

  Widget _joystickProperties(BuildContext context) {
    final strings = context.strings;
    final scheme = Theme.of(context).colorScheme;
    final place = _layout.joystick;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          strings.t('joystick'),
          style: Theme.of(context).textTheme.titleSmall
              ?.copyWith(fontWeight: FontWeight.w800),
        ),
        const SizedBox(height: 5),
        Text(
          strings.t('joystickHint'),
          style: Theme.of(context).textTheme.bodySmall
              ?.copyWith(color: scheme.onSurfaceVariant, height: 1.4),
        ),
        const SizedBox(height: 18),
        _sliderRow(
          context,
          label: strings.t('size'),
          valueLabel: '${(place.size * 100).round()}%',
          value: place.size.clamp(0.08, 0.45).toDouble(),
          min: 0.08,
          max: 0.45,
          onChanged: (value) => setState(() => place.size = value),
        ),
        _sliderRow(
          context,
          label: strings.t('opacity'),
          valueLabel: '${(place.opacity * 100).round()}%',
          value: place.opacity,
          min: 0.15,
          max: 1,
          onChanged: (value) => setState(() => place.opacity = value),
        ),
        const SizedBox(height: 8),
        Text(
          strings.t('canvasHint'),
          style: Theme.of(context).textTheme.labelSmall
              ?.copyWith(color: scheme.onSurfaceVariant, height: 1.45),
        ),
      ],
    );
  }

  Widget _buttonProperties(BuildContext context, TouchButton button) {
    final strings = context.strings;
    final scheme = Theme.of(context).colorScheme;
    final place = button.placement;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          strings.t('buttonLabel'),
          style: Theme.of(context).textTheme.titleSmall
              ?.copyWith(fontWeight: FontWeight.w800),
        ),
        const SizedBox(height: 8),
        TextField(
          controller: _labelController,
          maxLength: 32,
          onChanged: (value) {
            setState(() {
              button.label = value;
            });
          },
          decoration: InputDecoration(
            hintText: strings.t('newButton'),
            helperText: strings.t('buttonLabelHint'),
            counterText: '',
            prefixIcon: const Icon(Icons.text_fields),
          ),
        ),
        const SizedBox(height: 14),
        Row(
          children: [
            Expanded(
              child: Text(
                strings.t('keys'),
                style: Theme.of(context).textTheme.titleSmall
                    ?.copyWith(fontWeight: FontWeight.w800),
              ),
            ),
            TextButton.icon(
              onPressed: _editKeys,
              icon: const Icon(Icons.keyboard_rounded, size: 18),
              label: Text(strings.t('chooseKeys')),
            ),
          ],
        ),
        const SizedBox(height: 4),
        if (button.keys.isEmpty)
          Text(
            strings.t('noKeys'),
            style: Theme.of(context).textTheme.bodySmall
                ?.copyWith(color: scheme.error),
          )
        else
          Wrap(
            spacing: 6,
            runSpacing: 6,
            children: [
              for (final key in button.keys)
                InputChip(
                  label: Text(touchKeyLabel(key)),
                  onDeleted: () {
                    setState(() {
                      button.keys.remove(key);
                    });
                  },
                ),
            ],
          ),
        const SizedBox(height: 18),
        _sliderRow(
          context,
          label: strings.t('size'),
          valueLabel: '${(place.size * 100).round()}%',
          value: place.size.clamp(0.045, 0.45).toDouble(),
          min: 0.045,
          max: 0.45,
          onChanged: (value) => setState(() => place.size = value),
        ),
        _sliderRow(
          context,
          label: strings.t('opacity'),
          valueLabel: '${(place.opacity * 100).round()}%',
          value: place.opacity,
          min: 0.15,
          max: 1,
          onChanged: (value) => setState(() => place.opacity = value),
        ),
        const SizedBox(height: 18),
        SizedBox(
          width: double.infinity,
          child: OutlinedButton.icon(
            onPressed: _deleteButton,
            style: OutlinedButton.styleFrom(
              foregroundColor: scheme.error,
              side: BorderSide(color: scheme.error.withOpacity(0.45)),
            ),
            icon: const Icon(Icons.delete_outline_rounded, size: 19),
            label: Text(strings.t('deleteButton')),
          ),
        ),
        const SizedBox(height: 18),
        Text(
          strings.t('canvasHint'),
          style: Theme.of(context).textTheme.labelSmall
              ?.copyWith(color: scheme.onSurfaceVariant, height: 1.45),
        ),
      ],
    );
  }

  Widget _sliderRow(
    BuildContext context, {
    required String label,
    required String valueLabel,
    required double value,
    required double min,
    required double max,
    required ValueChanged<double> onChanged,
  }) {
    final scheme = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                label,
                style: Theme.of(context).textTheme.labelMedium
                    ?.copyWith(fontWeight: FontWeight.w600),
              ),
            ),
            Text(
              valueLabel,
              style: Theme.of(context).textTheme.labelMedium?.copyWith(
                color: scheme.primary,
                fontWeight: FontWeight.w800,
              ),
            ),
          ],
        ),
        Slider(
          value: value.clamp(min, max).toDouble(),
          min: min,
          max: max,
          onChanged: onChanged,
        ),
      ],
    );
  }
}

class _Canvas extends StatefulWidget {
  const _Canvas({
    required this.layout,
    required this.selected,
    required this.onSelect,
    required this.onChanged,
  });

  final TouchLayout layout;
  final String selected;
  final ValueChanged<String> onSelect;
  final VoidCallback onChanged;

  @override
  State<_Canvas> createState() => _CanvasState();
}

class _CanvasState extends State<_Canvas> {
  String? _activeId;
  bool _resizing = false;

  void _dragStart(String id, bool resizing) {
    widget.onSelect(id);
    setState(() {
      _activeId = id;
      _resizing = resizing;
    });
  }

  void _dragUpdate(DragUpdateDetails details, Size size, bool joystick) {
    if (_activeId == null) {
      return;
    }
    final placement = joystick
        ? widget.layout.joystick
        : widget.layout.buttons
              .firstWhere((button) => button.id == _activeId)
              .placement;
    if (_resizing) {
      final delta = (details.delta.dx + details.delta.dy) / (2 * size.height);
      placement.size = (placement.size + delta).clamp(0.045, 0.45).toDouble();
    } else {
      final halfY = (placement.size / 2).clamp(0.0, 0.5).toDouble();
      final halfX = (placement.size * size.height / size.width / 2)
          .clamp(0.0, 0.5)
          .toDouble();
      placement.x = (placement.x + details.delta.dx / size.width)
          .clamp(halfX, 1 - halfX)
          .toDouble();
      placement.y = (placement.y + details.delta.dy / size.height)
          .clamp(halfY, 1 - halfY)
          .toDouble();
    }
    widget.onChanged();
  }

  void _dragEnd() {
    setState(() {
      _activeId = null;
      _resizing = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return LayoutBuilder(
      builder: (context, constraints) {
        final size = Size(constraints.maxWidth, constraints.maxHeight);
        return GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: () => widget.onSelect(''),
          child: Stack(
            children: [
              Positioned.fill(
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    gradient: const LinearGradient(
                      begin: Alignment.topLeft,
                      end: Alignment.bottomRight,
                      colors: [Color(0xFF0D1410), Color(0xFF17241A)],
                    ),
                  ),
                ),
              ),
              Positioned.fill(
                child: CustomPaint(
                  painter: _GridPainter(scheme.primary.withOpacity(0.1)),
                ),
              ),
              Positioned(
                left: 16,
                top: 14,
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 6,
                  ),
                  decoration: BoxDecoration(
                    color: Colors.black.withOpacity(0.32),
                    borderRadius: BorderRadius.circular(10),
                    border: Border.all(color: Colors.white.withOpacity(0.12)),
                  ),
                  child: Text(
                    context.strings.t('editorCanvas'),
                    style: Theme.of(context).textTheme.labelSmall?.copyWith(
                      color: Colors.white70,
                      letterSpacing: 1.2,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
              ),
              _joystickItem(size),
              for (final button in widget.layout.buttons)
                _buttonItem(button, size),
            ],
          ),
        );
      },
    );
  }

  Widget _joystickItem(Size size) {
    final place = widget.layout.joystick;
    final px = (place.size * size.height)
        .clamp(54.0, size.height * 0.9)
        .toDouble();
    final left = place.x * size.width - px / 2;
    final top = place.y * size.height - px / 2;
    final selected = widget.selected == 'joystick';
    return Positioned(
      left: left,
      top: top,
      width: px,
      height: px,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () => widget.onSelect('joystick'),
        onPanStart: (details) {
          final handle = math.max(20.0, px * 0.32);
          _dragStart(
            'joystick',
            details.localPosition.dx > px - handle &&
                details.localPosition.dy > px - handle,
          );
        },
        onPanUpdate: (details) => _dragUpdate(details, size, true),
        onPanEnd: (_) => _dragEnd(),
        child: Container(
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: Colors.white.withOpacity(0.055),
            border: Border.all(
              color: selected
                  ? Theme.of(context).colorScheme.primary
                  : Colors.white.withOpacity(0.28),
              width: selected ? 2.2 : 1.2,
            ),
            boxShadow: [
              if (selected)
                BoxShadow(
                  color: Theme.of(context).colorScheme.primary.withOpacity(0.3),
                  blurRadius: 18,
                ),
            ],
          ),
          child: Stack(
            alignment: Alignment.center,
            children: [
              Container(
                width: px * 0.44,
                height: px * 0.44,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: Colors.white.withOpacity(0.16),
                  border: Border.all(color: Colors.white.withOpacity(0.36)),
                ),
                child: Icon(
                  Icons.gamepad_rounded,
                  size: px * 0.22,
                  color: Colors.white70,
                ),
              ),
              Positioned(
                left: 8,
                bottom: 8,
                child: Text(
                  'MOVE',
                  style: TextStyle(
                    color: Colors.white.withOpacity(0.55),
                    fontSize: math.min(11.0, px * 0.09),
                    fontWeight: FontWeight.w800,
                    letterSpacing: 1.4,
                  ),
                ),
              ),
              if (selected) _resizeHandle(),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buttonItem(TouchButton button, Size size) {
    final place = button.placement;
    final px = (place.size * size.height)
        .clamp(34.0, size.height * 0.9)
        .toDouble();
    final left = place.x * size.width - px / 2;
    final top = place.y * size.height - px / 2;
    final selected = widget.selected == button.id;
    final label = button.displayLabel;
    return Positioned(
      left: left,
      top: top,
      width: px,
      height: px,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () => widget.onSelect(button.id),
        onPanStart: (details) {
          final handle = math.max(20.0, px * 0.32);
          _dragStart(
            button.id,
            details.localPosition.dx > px - handle &&
                details.localPosition.dy > px - handle,
          );
        },
        onPanUpdate: (details) => _dragUpdate(details, size, false),
        onPanEnd: (_) => _dragEnd(),
        child: Container(
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.surface.withOpacity(0.18),
            borderRadius: BorderRadius.circular(px * 0.26),
            border: Border.all(
              color: selected
                  ? Theme.of(context).colorScheme.primary
                  : Colors.white.withOpacity(0.36),
              width: selected ? 2.2 : 1.2,
            ),
            boxShadow: [
              if (selected)
                BoxShadow(
                  color: Theme.of(context).colorScheme.primary
                      .withOpacity(0.35),
                  blurRadius: 16,
                ),
            ],
          ),
          child: Stack(
            children: [
              Positioned.fill(
                child: Padding(
                  padding: EdgeInsets.all(px * 0.1),
                  child: Center(
                    child: FittedBox(
                      fit: BoxFit.scaleDown,
                      child: Text(
                        label.isEmpty ? '·' : label,
                        maxLines: 2,
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          color: Colors.white.withOpacity(
                            label.isEmpty ? 0.45 : 0.94,
                          ),
                          fontSize: math.max(9.0, px * 0.15),
                          fontWeight: FontWeight.w700,
                          height: 1.15,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              Positioned(
                right: 2,
                top: 2,
                child: Icon(
                  Icons.open_in_full,
                  size: math.max(10.0, px * 0.12),
                  color: Colors.white.withOpacity(selected ? 0.8 : 0.25),
                ),
              ),
              if (selected) _resizeHandle(),
            ],
          ),
        ),
      ),
    );
  }

  Widget _resizeHandle() {
    final color = Theme.of(context).colorScheme.primary;
    return Positioned(
      right: 2,
      bottom: 2,
      child: Container(
        width: 20,
        height: 20,
        decoration: BoxDecoration(
          color: color,
          shape: BoxShape.circle,
          border: Border.all(color: Colors.white.withOpacity(0.65)),
        ),
        child: Icon(
          Icons.open_in_full,
          size: 11,
          color: Theme.of(context).colorScheme.onPrimary,
        ),
      ),
    );
  }
}

class _GridPainter extends CustomPainter {
  _GridPainter(this.color);

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = 0.7;
    const gap = 44.0;
    for (var x = gap; x < size.width; x += gap) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), paint);
    }
    for (var y = gap; y < size.height; y += gap) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), paint);
    }
    final center = Paint()
      ..color = Colors.white.withOpacity(0.2)
      ..strokeWidth = 1;
    final c = Offset(size.width / 2, size.height / 2);
    canvas.drawLine(c - const Offset(9, 0), c + const Offset(9, 0), center);
    canvas.drawLine(c - const Offset(0, 9), c + const Offset(0, 9), center);
  }

  @override
  bool shouldRepaint(covariant _GridPainter oldDelegate) {
    return oldDelegate.color != color;
  }
}

class _KeyPickerDialog extends StatefulWidget {
  const _KeyPickerDialog({required this.title, required this.initial});

  final String title;
  final List<String> initial;

  @override
  State<_KeyPickerDialog> createState() => _KeyPickerDialogState();
}

class _KeyPickerDialogState extends State<_KeyPickerDialog> {
  late final List<String> _selected;
  int _tab = 0;

  static const List<List<String>> _keyboardRows = [
    ['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'],
    ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'],
    ['z', 'x', 'c', 'v', 'b', 'n', 'm'],
    ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'],
    [
      'space',
      'left.shift',
      'left.control',
      'left.alt',
      'tab',
      'enter',
      'escape',
      'f1',
    ],
  ];

  @override
  void initState() {
    super.initState();
    _selected = List<String>.of(widget.initial.take(4));
  }

  void _toggle(String token) {
    setState(() {
      if (_selected.contains(token)) {
        _selected.remove(token);
      } else if (_selected.length < 4) {
        _selected.add(token);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final strings = context.strings;
    final scheme = Theme.of(context).colorScheme;
    final size = MediaQuery.of(context).size;
    final width = math.min(720.0, size.width - 32);
    final height = math.min(320.0, size.height - 24);
    final actions = <(String, String)>[
      ('action:key.attack', strings.t('actAttack')),
      ('action:key.use', strings.t('actUse')),
      ('action:key.jump', strings.t('actJump')),
      ('action:key.sneak', strings.t('actSneak')),
      ('action:key.sprint', strings.t('actSprint')),
      ('action:key.inventory', strings.t('actInventory')),
      ('action:key.chat', strings.t('actChat')),
      ('action:key.drop', strings.t('actDrop')),
      ('action:key.swapOffhand', strings.t('actSwap')),
      ('action:key.togglePerspective', strings.t('actPerspective')),
      ('key.keyboard.escape', strings.t('actPause')),
    ];
    return Dialog(
      insetPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      backgroundColor: scheme.surfaceContainerHigh,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(24)),
      child: SizedBox(
        width: width,
        height: height,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(
                      widget.title,
                      style: Theme.of(context).textTheme.titleMedium
                          ?.copyWith(fontWeight: FontWeight.w800),
                    ),
                  ),
                  Text(
                    '${_selected.length}/4',
                    style: Theme.of(context).textTheme.labelMedium
                        ?.copyWith(color: scheme.onSurfaceVariant),
                  ),
                  const SizedBox(width: 6),
                  IconButton(
                    onPressed: () => Navigator.of(context).pop(),
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints.tightFor(
                      width: 32,
                      height: 32,
                    ),
                    icon: const Icon(Icons.close_rounded, size: 19),
                  ),
                ],
              ),
              SegmentedButton<int>(
                style: const ButtonStyle(
                  visualDensity: VisualDensity.compact,
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                ),
                segments: [
                  ButtonSegment(
                    value: 0,
                    label: Text(strings.t('keyboard')),
                    icon: const Icon(Icons.keyboard_rounded, size: 15),
                  ),
                  ButtonSegment(
                    value: 1,
                    label: Text(strings.t('gameActions')),
                    icon: const Icon(Icons.sports_esports_rounded, size: 15),
                  ),
                ],
                selected: {_tab},
                showSelectedIcon: false,
                onSelectionChanged: (value) {
                  setState(() => _tab = value.first);
                },
              ),
              const SizedBox(height: 8),
              Expanded(
                child: _tab == 0
                    ? _keyboard(context)
                    : _gameActions(context, actions),
              ),
              if (_selected.isNotEmpty)
                Padding(
                  padding: const EdgeInsets.only(top: 4),
                  child: SizedBox(
                    height: 26.0,
                    child: ListView.separated(
                      scrollDirection: Axis.horizontal,
                      itemCount: _selected.length,
                      separatorBuilder: (_, __) => const SizedBox(width: 6),
                      itemBuilder: (context, index) {
                        final token = _selected[index];
                        return InputChip(
                          label: Text(touchKeyLabel(token)),
                          onDeleted: () => _toggle(token),
                          visualDensity: VisualDensity.compact,
                        );
                      },
                    ),
                  ),
                ),
              const SizedBox(height: 6),
              Row(
                children: [
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(),
                    child: Text(strings.t('cancel')),
                  ),
                  const Spacer(),
                  FilledButton.icon(
                    onPressed: _selected.isEmpty
                        ? null
                        : () => Navigator.of(context).pop(_selected),
                    icon: const Icon(Icons.check_rounded, size: 18),
                    label: Text(strings.t('done')),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _keyboard(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          for (final row in _keyboardRows)
            Padding(
              padding: const EdgeInsets.only(bottom: 3),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  for (final key in row)
                    _keyChip(
                      context,
                      'key.keyboard.$key',
                      _keyLabel(key),
                      scheme,
                    ),
                ],
              ),
            ),
        ],
      ),
    );
  }

  Widget _gameActions(BuildContext context, List<(String, String)> actions) {
    final scheme = Theme.of(context).colorScheme;
    return SingleChildScrollView(
      child: Wrap(
        spacing: 6,
        runSpacing: 6,
        children: [
          for (final action in actions)
            FilterChip(
              label: Text(action.$2),
              selected: _selected.contains(action.$1),
              onSelected: (_) => _toggle(action.$1),
              selectedColor: scheme.primary.withOpacity(0.24),
              checkmarkColor: scheme.primary,
            ),
        ],
      ),
    );
  }

  Widget _keyChip(
    BuildContext context,
    String token,
    String label,
    ColorScheme scheme,
  ) {
    final selected = _selected.contains(token);
    final long = label.length > 2;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 2),
      child: SizedBox(
        width: long ? 54.0 : 32.0,
        height: 24.0,
        child: Material(
          color: selected
              ? scheme.primary.withOpacity(0.9)
              : scheme.surfaceContainerHighest,
          borderRadius: BorderRadius.circular(8),
          child: InkWell(
            borderRadius: BorderRadius.circular(8),
            onTap: () => _toggle(token),
            child: Center(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  color: selected ? scheme.onPrimary : scheme.onSurface,
                  fontSize: 10,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  String _keyLabel(String key) {
    switch (key) {
      case 'space':
        return 'Space';
      case 'left.shift':
        return 'Shift';
      case 'left.control':
        return 'Ctrl';
      case 'left.alt':
        return 'Alt';
      case 'tab':
        return 'Tab';
      case 'enter':
        return 'Enter';
      case 'escape':
        return 'Esc';
      case 'f1':
        return 'F1';
    }
    return key.toUpperCase();
  }
}
