import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import 'core/app_state.dart';
import 'core/app_strings.dart';
import 'core/theme.dart';
import 'ui/shell.dart';
import 'ui/widgets/logo_mark.dart';

class HorizonApp extends StatelessWidget {
  const HorizonApp({super.key, required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<int>(
      valueListenable: state.appearanceRevision,
      builder: (context, _, __) {
        final settings = state.settings;
        final locale = settings.locale == 'system'
            ? null
            : Locale(settings.locale);
        final mode = switch (settings.theme) {
          'light' => ThemeMode.light,
          'dark' => ThemeMode.dark,
          _ => ThemeMode.system,
        };
        final accent = accentById(settings.accent);
        return MaterialApp(
          debugShowCheckedModeBanner: false,
          title: 'Horizon',
          locale: locale,
          supportedLocales: const [Locale('tr'), Locale('en')],
          localizationsDelegates: const [
            AppStrings.delegate,
            GlobalMaterialLocalizations.delegate,
            GlobalWidgetsLocalizations.delegate,
            GlobalCupertinoLocalizations.delegate,
          ],
          themeMode: mode,
          theme: buildHorizonTheme(Brightness.light, accent),
          darkTheme: buildHorizonTheme(Brightness.dark, accent),
          themeAnimationDuration: const Duration(milliseconds: 420),
          themeAnimationCurve: Curves.easeOutCubic,
          home: _Root(state: state),
        );
      },
    );
  }
}

class _Root extends StatelessWidget {
  const _Root({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final style =
        (theme.brightness == Brightness.dark
                ? SystemUiOverlayStyle.light
                : SystemUiOverlayStyle.dark)
            .copyWith(
              statusBarColor: Colors.transparent,
              systemNavigationBarColor: Colors.transparent,
              systemNavigationBarDividerColor: Colors.transparent,
              statusBarIconBrightness: theme.brightness == Brightness.dark
                  ? Brightness.light
                  : Brightness.dark,
              systemNavigationBarIconBrightness:
                  theme.brightness == Brightness.dark
                  ? Brightness.light
                  : Brightness.dark,
            );
    return AnnotatedRegion<SystemUiOverlayStyle>(
      value: style,
      child: AnimatedBuilder(
        animation: state,
        builder: (context, _) {
          if (state.fatalError != null) {
            return _FatalScreen(message: state.fatalError!);
          }
          if (state.initializing) {
            return const _BootScreen();
          }
          return LauncherShell(state: state);
        },
      ),
    );
  }
}

class _BootScreen extends StatelessWidget {
  const _BootScreen();

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      body: Center(
        child: TweenAnimationBuilder<double>(
          tween: Tween(begin: 0.86, end: 1),
          duration: const Duration(milliseconds: 700),
          curve: Curves.easeOutCubic,
          builder: (context, value, child) {
            return Opacity(
              opacity: value,
              child: Transform.scale(scale: value, child: child),
            );
          },
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const LogoMark(size: 64),
              const SizedBox(height: 18),
              Text(
                'HORIZON',
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                  letterSpacing: 5,
                  fontWeight: FontWeight.w800,
                  color: scheme.onSurface,
                ),
              ),
              const SizedBox(height: 20),
              SizedBox(
                width: 22,
                height: 22,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  color: scheme.primary,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _FatalScreen extends StatelessWidget {
  const _FatalScreen({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final strings = context.strings;
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(28),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Spacer(),
              const LogoMark(size: 56),
              const SizedBox(height: 28),
              Text(
                strings.t('somethingWentWrong'),
                style: Theme.of(context).textTheme.headlineMedium,
              ),
              const SizedBox(height: 12),
              Text(
                message,
                style: Theme.of(context).textTheme.bodyMedium
                    ?.copyWith(color: scheme.onSurfaceVariant),
              ),
              const SizedBox(height: 32),
              FilledButton(
                onPressed: () => SystemNavigator.pop(),
                child: Text(strings.t('close')),
              ),
              const Spacer(flex: 2),
            ],
          ),
        ),
      ),
    );
  }
}
