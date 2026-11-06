import 'package:flutter/material.dart';

class LogoMark extends StatelessWidget {
  const LogoMark({super.key, this.size = 42});

  final double size;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: size,
      height: size,
      child: CustomPaint(painter: _LogoPainter(Theme.of(context).colorScheme)),
    );
  }
}

class _LogoPainter extends CustomPainter {
  _LogoPainter(this.scheme);

  final ColorScheme scheme;

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    final radius = Radius.circular(size.width * 0.27);
    final background = Paint()
      ..shader = LinearGradient(
        begin: Alignment.topLeft,
        end: Alignment.bottomRight,
        colors: [
          scheme.primary,
          Color.alphaBlend(scheme.secondary.withOpacity(0.7), scheme.primary),
        ],
      ).createShader(rect);
    canvas.drawRRect(RRect.fromRectAndRadius(rect, radius), background);

    final cell = size.width * 0.25;
    final gap = size.width * 0.055;
    final left = (size.width - cell * 2 - gap) / 2;
    final top = (size.height - cell * 2 - gap) / 2;
    final light = Paint()..color = scheme.onPrimary.withOpacity(0.92);
    final dark = Paint()..color = scheme.onPrimary.withOpacity(0.22);
    final corner = Radius.circular(cell * 0.2);

    void block(double x, double y, Paint paint) {
      canvas.drawRRect(
        RRect.fromRectAndRadius(Rect.fromLTWH(x, y, cell, cell), corner),
        paint,
      );
    }

    block(left, top, light);
    block(left + cell + gap, top, dark);
    block(left, top + cell + gap, dark);
    block(left + cell + gap, top + cell + gap, light);

    final highlight = Paint()
      ..shader = LinearGradient(
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
        colors: [Colors.white.withOpacity(0.18), Colors.transparent],
      ).createShader(Rect.fromLTWH(0, 0, size.width, size.height * 0.5));
    canvas.drawRRect(RRect.fromRectAndRadius(rect, radius), highlight);
  }

  @override
  bool shouldRepaint(covariant _LogoPainter oldDelegate) =>
      oldDelegate.scheme != scheme;
}
