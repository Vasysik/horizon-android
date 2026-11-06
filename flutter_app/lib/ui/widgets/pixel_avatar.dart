import 'package:flutter/material.dart';

class PixelAvatar extends StatelessWidget {
  const PixelAvatar({super.key, required this.name, this.size = 44});

  final String name;
  final double size;

  @override
  Widget build(BuildContext context) {
    final hash = _hash(name.isEmpty ? 'Steve' : name);
    final hue = (hash % 360).toDouble();
    final skin = HSLColor.fromAHSL(1, hue, 0.36, 0.66).toColor();
    final skinShade = HSLColor.fromAHSL(1, hue, 0.42, 0.53).toColor();
    final hair = HSLColor.fromAHSL(1, (hue + 28) % 360, 0.45, 0.25).toColor();
    final eye = HSLColor.fromAHSL(1, hue, 0.5, 0.16).toColor();
    return SizedBox(
      width: size,
      height: size,
      child: ClipRRect(
        borderRadius: BorderRadius.circular(size * 0.32),
        child: CustomPaint(
          painter: _AvatarPainter(
            skin: skin,
            skinShade: skinShade,
            hair: hair,
            eye: eye,
          ),
        ),
      ),
    );
  }

  int _hash(String value) {
    var result = 7;
    for (final unit in value.codeUnits) {
      result = (result * 31 + unit) & 0x7fffffff;
    }
    return result;
  }
}

class _AvatarPainter extends CustomPainter {
  _AvatarPainter({
    required this.skin,
    required this.skinShade,
    required this.hair,
    required this.eye,
  });

  final Color skin;
  final Color skinShade;
  final Color hair;
  final Color eye;

  @override
  void paint(Canvas canvas, Size size) {
    final background = Paint();
    background.shader = LinearGradient(
      begin: Alignment.topLeft,
      end: Alignment.bottomRight,
      colors: [skinShade, Color.alphaBlend(hair.withOpacity(0.35), skinShade)],
    ).createShader(Offset.zero & size);
    canvas.drawRect(Offset.zero & size, background);

    final cell = size.width / 6;
    final face = Paint()..color = skin;
    final shade = Paint()..color = skinShade;
    final hairPaint = Paint()..color = hair;
    final eyePaint = Paint()..color = eye;
    final light = Paint()..color = Colors.white.withOpacity(0.2);

    void block(int x, int y, Paint paint) {
      canvas.drawRect(
        Rect.fromLTWH(x * cell, y * cell, cell + 0.4, cell + 0.4),
        paint,
      );
    }

    for (var x = 0; x < 6; x++) {
      block(x, 0, hairPaint);
      block(x, 5, shade);
    }
    for (var y = 1; y < 5; y++) {
      block(0, y, hairPaint);
      block(5, y, hairPaint);
    }
    for (var y = 1; y < 5; y++) {
      for (var x = 1; x < 5; x++) {
        block(x, y, face);
      }
    }
    block(1, 1, light);
    block(2, 1, light);
    block(1, 3, eyePaint);
    block(4, 3, eyePaint);
    block(2, 4, shade);
    block(3, 4, shade);
    final border = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.2
      ..color = Colors.white.withOpacity(0.14);
    canvas.drawRect(Offset.zero & size, border);
  }

  @override
  bool shouldRepaint(covariant _AvatarPainter oldDelegate) {
    return oldDelegate.skin != skin ||
        oldDelegate.skinShade != skinShade ||
        oldDelegate.hair != hair ||
        oldDelegate.eye != eye;
  }
}
