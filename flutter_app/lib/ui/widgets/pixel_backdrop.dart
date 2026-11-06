import 'dart:math' as math;

import 'package:flutter/material.dart';

class PixelBackdrop extends StatelessWidget {
  const PixelBackdrop({super.key});

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Positioned.fill(
      child: IgnorePointer(
        child: RepaintBoundary(
          child: CustomPaint(painter: _BackdropPainter(scheme)),
        ),
      ),
    );
  }
}

class _BackdropPainter extends CustomPainter {
  _BackdropPainter(this.scheme);

  final ColorScheme scheme;

  @override
  void paint(Canvas canvas, Size size) {
    final glowRect = Rect.fromCircle(
      center: Offset(size.width * 0.82, size.height * 0.08),
      radius: size.width * 0.72,
    );
    final glow = Paint()
      ..shader = RadialGradient(
        colors: [
          scheme.primary.withOpacity(0.14),
          scheme.primary.withOpacity(0.03),
          Colors.transparent,
        ],
        stops: const [0, 0.45, 1],
      ).createShader(glowRect);
    canvas.drawCircle(glowRect.center, glowRect.width / 2, glow);

    final starPaint = Paint()..color = scheme.onSurface.withOpacity(0.18);
    final random = math.Random(17);
    for (var i = 0; i < 26; i++) {
      final x = random.nextDouble() * size.width;
      final y = random.nextDouble() * size.height * 0.42;
      final radius = random.nextInt(3) == 0 ? 1.3 : 0.8;
      canvas.drawCircle(Offset(x, y), radius, starPaint);
    }

    _hills(canvas, size, scheme.primary.withOpacity(0.055), 0.72, 27, 3);
    _hills(canvas, size, scheme.primary.withOpacity(0.045), 0.8, 38, 9);
    _hills(
      canvas,
      size,
      scheme.surfaceContainerHigh.withOpacity(0.55),
      0.9,
      52,
      5,
    );
  }

  void _hills(
    Canvas canvas,
    Size size,
    Color color,
    double baseline,
    double block,
    int seed,
  ) {
    final paint = Paint()..color = color;
    final random = math.Random(seed);
    final path = Path()..moveTo(0, size.height);
    var x = 0.0;
    var index = 0;
    while (x < size.width) {
      final height =
          (index.isEven ? random.nextDouble() : random.nextDouble() * 0.6) *
          size.height *
          0.09;
      final top = size.height * baseline - height;
      path.lineTo(x, top);
      path.lineTo(math.min(x + block, size.width), top);
      x += block;
      index++;
    }
    path.lineTo(size.width, size.height);
    path.close();
    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(covariant _BackdropPainter oldDelegate) {
    return oldDelegate.scheme != scheme;
  }
}
