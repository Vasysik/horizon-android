import 'package:flutter/material.dart';

class Accent {
  const Accent(
    this.id,
    this.labelKey,
    this.seed,
    this.darkPrimary,
    this.lightPrimary,
  );

  final String id;
  final String labelKey;
  final Color seed;
  final Color darkPrimary;
  final Color lightPrimary;
}

const List<Accent> accents = [
  Accent(
    'grass',
    'accentGrass',
    Color(0xFF74B24A),
    Color(0xFFA8D98A),
    Color(0xFF3E6B27),
  ),
  Accent(
    'amethyst',
    'accentAmethyst',
    Color(0xFF9B7BD4),
    Color(0xFFD0BCFF),
    Color(0xFF5E408F),
  ),
  Accent(
    'ocean',
    'accentOcean',
    Color(0xFF4C9BB8),
    Color(0xFF8ED0E5),
    Color(0xFF1F657C),
  ),
  Accent(
    'ember',
    'accentEmber',
    Color(0xFFD27A4C),
    Color(0xFFFFB68E),
    Color(0xFF98451F),
  ),
  Accent(
    'gold',
    'accentGold',
    Color(0xFFC9A45C),
    Color(0xFFE8CE96),
    Color(0xFF70551C),
  ),
];

Accent accentById(String id) {
  return accents.firstWhere(
    (accent) => accent.id == id,
    orElse: () => accents.first,
  );
}

ThemeData buildHorizonTheme(Brightness brightness, Accent accent) {
  final isDark = brightness == Brightness.dark;
  final base = ColorScheme.fromSeed(
    seedColor: accent.seed,
    brightness: brightness,
  );
  final scheme = base.copyWith(
    primary: isDark ? accent.darkPrimary : accent.lightPrimary,
    onPrimary: isDark ? const Color(0xFF0D1A08) : Colors.white,
    primaryContainer: isDark
        ? Color.alphaBlend(
            accent.darkPrimary.withOpacity(0.18),
            const Color(0xFF172015),
          )
        : Color.alphaBlend(
            accent.lightPrimary.withOpacity(0.14),
            const Color(0xFFF2F6EC),
          ),
    onPrimaryContainer: isDark
        ? const Color(0xFFD6F5BF)
        : const Color(0xFF172A0D),
    secondary: isDark ? const Color(0xFFD9C08C) : const Color(0xFF6D5824),
    onSecondary: isDark ? const Color(0xFF2A210B) : Colors.white,
    secondaryContainer: isDark
        ? const Color(0xFF3A3120)
        : const Color(0xFFF0E3C4),
    onSecondaryContainer: isDark
        ? const Color(0xFFF1E1B8)
        : const Color(0xFF251D08),
    tertiary: isDark ? const Color(0xFF8FC7D8) : const Color(0xFF256B7C),
    onTertiary: isDark ? const Color(0xFF06232B) : Colors.white,
    surface: isDark ? const Color(0xFF0C110B) : const Color(0xFFF7F9F2),
    onSurface: isDark ? const Color(0xFFE3E9DC) : const Color(0xFF171C14),
    onSurfaceVariant: isDark
        ? const Color(0xFFAEB9A6)
        : const Color(0xFF4E5849),
    surfaceContainerLowest: isDark
        ? const Color(0xFF080C07)
        : const Color(0xFFFFFFFF),
    surfaceContainerLow: isDark
        ? const Color(0xFF11170F)
        : const Color(0xFFF1F4EB),
    surfaceContainer: isDark
        ? const Color(0xFF151C13)
        : const Color(0xFFEBEFE4),
    surfaceContainerHigh: isDark
        ? const Color(0xFF1A2218)
        : const Color(0xFFE4E9DC),
    surfaceContainerHighest: isDark
        ? const Color(0xFF212A1F)
        : const Color(0xFFDDE3D4),
    outline: isDark ? const Color(0xFF3A4536) : const Color(0xFFB4BFA9),
    outlineVariant: isDark ? const Color(0xFF283124) : const Color(0xFFD0D8C7),
    error: isDark ? const Color(0xFFFFB4AB) : const Color(0xFFBA1A1A),
    onError: isDark ? const Color(0xFF690005) : Colors.white,
    errorContainer: isDark ? const Color(0xFF3E1210) : const Color(0xFFFFDAD6),
    onErrorContainer: isDark
        ? const Color(0xFFFFDAD6)
        : const Color(0xFF410002),
  );

  final textTheme = _horizonTextTheme(
    ThemeData(brightness: brightness).textTheme,
  );

  return ThemeData(
    useMaterial3: true,
    brightness: brightness,
    colorScheme: scheme,
    scaffoldBackgroundColor: scheme.surface,
    textTheme: textTheme,
    splashFactory: InkRipple.splashFactory,
    visualDensity: VisualDensity.standard,
    pageTransitionsTheme: PageTransitionsTheme(
      builders: {
        TargetPlatform.android: const HorizonPageTransitionsBuilder(),
        TargetPlatform.iOS: const CupertinoPageTransitionsBuilder(),
        TargetPlatform.macOS: const CupertinoPageTransitionsBuilder(),
      },
    ),
    navigationBarTheme: NavigationBarThemeData(
      height: 74,
      elevation: 0,
      backgroundColor: Colors.transparent,
      indicatorColor: scheme.primary.withOpacity(0.18),
      labelBehavior: NavigationDestinationLabelBehavior.alwaysShow,
      labelTextStyle: WidgetStatePropertyAll(
        textTheme.labelMedium?.copyWith(fontWeight: FontWeight.w700) ??
            const TextStyle(),
      ),
      iconTheme: WidgetStateProperty.resolveWith((states) {
        final selected = states.contains(WidgetState.selected);
        return IconThemeData(
          size: 24,
          color: selected ? scheme.primary : scheme.onSurfaceVariant,
        );
      }),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        minimumSize: const Size(64, 54),
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(18)),
        textStyle: textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
        textStyle: textTheme.labelLarge?.copyWith(fontWeight: FontWeight.w700),
      ),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: IconButton.styleFrom(
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      ),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: scheme.surfaceContainerHigh.withOpacity(0.72),
      contentPadding: const EdgeInsets.symmetric(horizontal: 18, vertical: 16),
      hintStyle: textTheme.bodyMedium?.copyWith(
        color: scheme.onSurfaceVariant.withOpacity(0.72),
      ),
      labelStyle: textTheme.bodyMedium?.copyWith(
        color: scheme.onSurfaceVariant,
      ),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(18),
        borderSide: BorderSide.none,
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(18),
        borderSide: BorderSide(color: scheme.outlineVariant.withOpacity(0.55)),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(18),
        borderSide: BorderSide(
          color: scheme.primary.withOpacity(0.75),
          width: 1.4,
        ),
      ),
      errorBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(18),
        borderSide: BorderSide(color: scheme.error),
      ),
    ),
    dividerTheme: DividerThemeData(
      color: scheme.outlineVariant.withOpacity(0.5),
      thickness: 1,
      space: 1,
    ),
    snackBarTheme: SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      backgroundColor: scheme.inverseSurface,
      contentTextStyle: textTheme.bodyMedium?.copyWith(
        color: scheme.onInverseSurface,
      ),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      insetPadding: const EdgeInsets.fromLTRB(16, 0, 16, 108),
    ),
    progressIndicatorTheme: ProgressIndicatorThemeData(
      color: scheme.primary,
      linearTrackColor: scheme.surfaceContainerHighest,
      circularTrackColor: scheme.surfaceContainerHighest,
    ),
    listTileTheme: ListTileThemeData(
      iconColor: scheme.onSurfaceVariant,
      textColor: scheme.onSurface,
      contentPadding: const EdgeInsets.symmetric(horizontal: 18, vertical: 4),
    ),
    switchTheme: SwitchThemeData(
      thumbColor: WidgetStateProperty.resolveWith((states) {
        return states.contains(WidgetState.selected)
            ? scheme.onPrimary
            : scheme.outline;
      }),
      trackColor: WidgetStateProperty.resolveWith((states) {
        return states.contains(WidgetState.selected)
            ? scheme.primary
            : scheme.surfaceContainerHighest;
      }),
      trackOutlineColor: WidgetStatePropertyAll(
        scheme.outlineVariant.withOpacity(0.7),
      ),
    ),
    sliderTheme: SliderThemeData(
      activeTrackColor: scheme.primary,
      inactiveTrackColor: scheme.surfaceContainerHighest,
      thumbColor: scheme.primary,
    ),
  );
}

TextTheme _horizonTextTheme(TextTheme base) {
  return base.copyWith(
    displaySmall: base.displaySmall?.copyWith(
      fontWeight: FontWeight.w800,
      letterSpacing: -1.4,
      height: 1.02,
    ),
    headlineMedium: base.headlineMedium?.copyWith(
      fontWeight: FontWeight.w800,
      letterSpacing: -0.8,
      height: 1.1,
    ),
    headlineSmall: base.headlineSmall?.copyWith(
      fontWeight: FontWeight.w700,
      letterSpacing: -0.4,
    ),
    titleLarge: base.titleLarge?.copyWith(
      fontWeight: FontWeight.w700,
      letterSpacing: -0.2,
    ),
    titleMedium: base.titleMedium?.copyWith(fontWeight: FontWeight.w600),
    bodyLarge: base.bodyLarge?.copyWith(height: 1.45),
    bodyMedium: base.bodyMedium?.copyWith(height: 1.45),
    labelLarge: base.labelLarge?.copyWith(
      fontWeight: FontWeight.w700,
      letterSpacing: 0.2,
    ),
    labelMedium: base.labelMedium?.copyWith(fontWeight: FontWeight.w600),
    labelSmall: base.labelSmall?.copyWith(
      fontWeight: FontWeight.w700,
      letterSpacing: 1.2,
    ),
  );
}

class HorizonPageTransitionsBuilder extends PageTransitionsBuilder {
  const HorizonPageTransitionsBuilder();

  @override
  Widget buildTransitions<T>(
    PageRoute<T> route,
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    final primary = CurvedAnimation(
      parent: animation,
      curve: Curves.easeOutCubic,
      reverseCurve: Curves.easeInCubic,
    );
    final secondary = CurvedAnimation(
      parent: secondaryAnimation,
      curve: Curves.easeOutCubic,
      reverseCurve: Curves.easeInCubic,
    );
    return FadeTransition(
      opacity: Tween<double>(begin: 0, end: 1).animate(primary),
      child: SlideTransition(
        position: Tween<Offset>(
          begin: const Offset(0.04, 0.015),
          end: Offset.zero,
        ).animate(primary),
        child: SlideTransition(
          position: Tween<Offset>(
            begin: Offset.zero,
            end: const Offset(-0.015, 0),
          ).animate(secondary),
          child: child,
        ),
      ),
    );
  }
}

class HorizonRadius {
  static const double card = 26;
  static const double sheet = 32;
  static const double pill = 999;
}
