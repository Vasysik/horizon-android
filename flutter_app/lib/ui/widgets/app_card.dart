import 'package:flutter/material.dart';

class AppCard extends StatelessWidget {
  const AppCard({
    super.key,
    required this.child,
    this.padding = const EdgeInsets.all(20),
    this.onTap,
    this.color,
    this.gradient,
    this.radius = 26,
    this.borderColor,
    this.margin,
  });

  final Widget child;
  final EdgeInsetsGeometry padding;
  final VoidCallback? onTap;
  final Color? color;
  final Gradient? gradient;
  final double radius;
  final Color? borderColor;
  final EdgeInsetsGeometry? margin;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final decoration = BoxDecoration(
      color: gradient == null ? (color ?? scheme.surfaceContainerLow) : null,
      gradient: gradient,
      borderRadius: BorderRadius.circular(radius),
      border: Border.all(
        color: borderColor ?? scheme.outlineVariant.withOpacity(0.5),
        width: 1,
      ),
      boxShadow: [
        BoxShadow(
          color: Colors.black.withOpacity(0.08),
          blurRadius: 24,
          offset: const Offset(0, 12),
        ),
      ],
    );
    final content = Padding(padding: padding, child: child);
    final card = Material(
      color: Colors.transparent,
      child: Ink(
        decoration: decoration,
        child: onTap == null
            ? content
            : InkWell(
                borderRadius: BorderRadius.circular(radius),
                onTap: onTap,
                child: content,
              ),
      ),
    );
    if (margin == null) {
      return ClipRRect(
        borderRadius: BorderRadius.circular(radius),
        child: card,
      );
    }
    return Padding(
      padding: margin!,
      child: ClipRRect(
        borderRadius: BorderRadius.circular(radius),
        child: card,
      ),
    );
  }
}
