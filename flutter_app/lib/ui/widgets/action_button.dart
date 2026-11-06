import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

enum ActionTone { primary, tonal, ghost }

class ActionButton extends StatefulWidget {
  const ActionButton({
    super.key,
    required this.label,
    required this.onPressed,
    this.icon,
    this.loading = false,
    this.tone = ActionTone.primary,
    this.expand = true,
    this.height = 58,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;
  final bool loading;
  final ActionTone tone;
  final bool expand;
  final double height;

  @override
  State<ActionButton> createState() => _ActionButtonState();
}

class _ActionButtonState extends State<ActionButton> {
  bool _pressed = false;

  bool get _enabled => widget.onPressed != null && !widget.loading;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final style = switch (widget.tone) {
      ActionTone.primary => (
        scheme.onPrimary,
        BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: [
              scheme.primary,
              Color.alphaBlend(
                scheme.secondary.withOpacity(0.38),
                scheme.primary,
              ),
            ],
          ),
          borderRadius: BorderRadius.circular(20),
          boxShadow: [
            BoxShadow(
              color: scheme.primary.withOpacity(0.22),
              blurRadius: 24,
              offset: const Offset(0, 10),
            ),
          ],
        ),
      ),
      ActionTone.tonal => (
        scheme.onSurface,
        BoxDecoration(
          color: scheme.surfaceContainerHigh.withOpacity(0.9),
          borderRadius: BorderRadius.circular(20),
          border: Border.all(color: scheme.outlineVariant.withOpacity(0.55)),
        ),
      ),
      ActionTone.ghost => (
        scheme.onSurfaceVariant,
        BoxDecoration(
          color: Colors.transparent,
          borderRadius: BorderRadius.circular(20),
          border: Border.all(color: scheme.outlineVariant.withOpacity(0.45)),
        ),
      ),
    };
    final foreground = style.$1;
    final decoration = style.$2;

    final content = Container(
      height: widget.height,
      padding: const EdgeInsets.symmetric(horizontal: 22),
      decoration: decoration,
      child: Row(
        mainAxisSize: widget.expand ? MainAxisSize.max : MainAxisSize.min,
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          if (widget.loading)
            SizedBox(
              width: 19,
              height: 19,
              child: CircularProgressIndicator(
                strokeWidth: 2.2,
                color: foreground,
              ),
            )
          else if (widget.icon != null)
            Icon(widget.icon, size: 22, color: foreground),
          if (widget.loading || widget.icon != null) const SizedBox(width: 10),
          Flexible(
            child: Text(
              widget.label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textTheme.titleMedium?.copyWith(
                color: foreground,
                fontWeight: FontWeight.w700,
                letterSpacing: -0.1,
              ),
            ),
          ),
        ],
      ),
    );

    return Semantics(
      button: true,
      enabled: _enabled,
      label: widget.label,
      child: AnimatedOpacity(
        duration: const Duration(milliseconds: 180),
        opacity: _enabled ? 1 : 0.55,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapDown: _enabled ? (_) => setState(() => _pressed = true) : null,
          onTapCancel: _enabled ? () => setState(() => _pressed = false) : null,
          onTapUp: _enabled
              ? (_) {
                  setState(() => _pressed = false);
                  HapticFeedback.lightImpact();
                  widget.onPressed?.call();
                }
              : null,
          child: AnimatedScale(
            duration: const Duration(milliseconds: 140),
            curve: Curves.easeOutCubic,
            scale: _pressed ? 0.975 : 1,
            child: content,
          ),
        ),
      ),
    );
  }
}
