import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../core/app_state.dart';
import '../core/app_strings.dart';
import 'download_sheet.dart';
import 'widgets/action_button.dart';
import 'widgets/app_card.dart';
import 'widgets/logo_mark.dart';
import 'widgets/pixel_avatar.dart';
import 'widgets/pixel_backdrop.dart';
import 'widgets/stat_card.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key, required this.state, required this.onNavigate});

  final AppState state;
  final ValueChanged<int> onNavigate;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Stack(
        children: [
          const PixelBackdrop(),
          SafeArea(
            bottom: false,
            child: RefreshIndicator(
              color: Theme.of(context).colorScheme.primary,
              onRefresh: state.refresh,
              child: CustomScrollView(
                physics: const BouncingScrollPhysics(
                  parent: AlwaysScrollableScrollPhysics(),
                ),
                slivers: [
                  SliverPadding(
                    padding: const EdgeInsets.fromLTRB(20, 18, 20, 128),
                    sliver: SliverList(
                      delegate: SliverChildListDelegate([
                        _Header(
                          name: state.displayName,
                          onProfile: () => onNavigate(1),
                        ),
                        const SizedBox(height: 22),
                        _PlayCard(state: state),
                        const SizedBox(height: 12),
                        _StatsRow(state: state),
                        const SizedBox(height: 28),
                        const _Footer(),
                      ]),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.name, required this.onProfile});

  final String name;
  final VoidCallback onProfile;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final strings = context.strings;
    return Row(
      children: [
        const LogoMark(size: 46),
        const SizedBox(width: 13),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                strings.t('appName'),
                style: textTheme.titleLarge?.copyWith(
                  fontWeight: FontWeight.w800,
                  letterSpacing: 0.4,
                ),
              ),
              const SizedBox(height: 1),
              Text(
                strings.t('appTagline'),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: textTheme.bodySmall?.copyWith(
                  color: scheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
        const SizedBox(width: 10),
        Material(
          color: Colors.transparent,
          child: InkWell(
            borderRadius: BorderRadius.circular(18),
            onTap: onProfile,
            child: Container(
              padding: const EdgeInsets.fromLTRB(6, 6, 12, 6),
              decoration: BoxDecoration(
                color: scheme.surfaceContainerHigh.withOpacity(0.78),
                borderRadius: BorderRadius.circular(18),
                border: Border.all(
                  color: scheme.outlineVariant.withOpacity(0.55),
                ),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  PixelAvatar(name: name, size: 30),
                  const SizedBox(width: 9),
                  ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 88),
                    child: Text(
                      name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: textTheme.labelLarge?.copyWith(fontSize: 12.5),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class _PlayCard extends StatelessWidget {
  const _PlayCard({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final strings = context.strings;
    final status = state.status;
    final failed = status?.failed ?? false;
    final downloading = status?.running ?? false;
    final installed = state.installed;
    final stalled = state.stalled;

    final buttonLabel = installed
        ? strings.t('play')
        : downloading
        ? (stalled ? strings.t('resume') : strings.t('downloadDetails'))
        : failed
        ? strings.t('retry')
        : strings.t('downloadGame');
    final buttonIcon = installed
        ? Icons.play_arrow_rounded
        : downloading
        ? (stalled ? Icons.refresh_rounded : Icons.expand_more_rounded)
        : failed
        ? Icons.refresh_rounded
        : Icons.download_rounded;

    void action() {
      if (installed) {
        state.launchGame();
      } else if (downloading) {
        if (stalled) {
          state.resumeDownload();
        } else {
          showDownloadSheet(context, state);
        }
      } else {
        state.startDownload();
      }
    }

    final hint = installed
        ? (state.settings.server.trim().isEmpty
              ? strings.t('playSingleplayer')
              : strings
                    .t('playWithServer')
                    .replaceFirst('{server}', state.settings.server.trim()))
        : downloading
        ? strings.t('downloadBackgroundHint')
        : strings.t('downloadHint');

    return AppCard(
      padding: EdgeInsets.zero,
      radius: 30,
      gradient: LinearGradient(
        begin: Alignment.topLeft,
        end: Alignment.bottomRight,
        colors: [
          scheme.surfaceContainerHigh,
          Color.alphaBlend(
            scheme.primary.withOpacity(0.055),
            scheme.surfaceContainerLow,
          ),
        ],
      ),
      borderColor: scheme.outlineVariant.withOpacity(0.6),
      child: Stack(
        children: [
          Positioned.fill(
            child: IgnorePointer(
              child: ClipRRect(
                borderRadius: BorderRadius.circular(30),
                child: CustomPaint(painter: _PlayScenePainter(scheme)),
              ),
            ),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(22, 20, 22, 22),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Align(
                  alignment: Alignment.centerRight,
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 11,
                      vertical: 6,
                    ),
                    decoration: BoxDecoration(
                      border: Border.all(
                        color: scheme.outlineVariant.withOpacity(0.6),
                      ),
                      borderRadius: BorderRadius.circular(999),
                    ),
                    child: Text(
                      strings.t('versionNumber'),
                      style: textTheme.labelSmall?.copyWith(
                        color: scheme.onSurfaceVariant,
                        letterSpacing: 1,
                      ),
                    ),
                  ),
                ),
                const SizedBox(height: 26),
                Text(
                  strings.t('versionLabel'),
                  style: textTheme.labelSmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                    letterSpacing: 2.2,
                  ),
                ),
                const SizedBox(height: 3),
                Text(
                  strings.t('versionNumber'),
                  style: textTheme.displaySmall?.copyWith(
                    color: scheme.primary,
                    fontSize: 58,
                    fontWeight: FontWeight.w800,
                  ),
                ),
                if (failed) ...[
                  const SizedBox(height: 16),
                  Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: scheme.errorContainer.withOpacity(0.5),
                      borderRadius: BorderRadius.circular(14),
                      border: Border.all(color: scheme.error.withOpacity(0.22)),
                    ),
                    child: Row(
                      children: [
                        Icon(
                          Icons.error_outline_rounded,
                          size: 18,
                          color: scheme.error,
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Text(
                            strings.t('downloadFailedHint'),
                            style: textTheme.bodySmall?.copyWith(
                              color: scheme.onErrorContainer,
                              height: 1.35,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
                const SizedBox(height: 22),
                ActionButton(
                  label: buttonLabel,
                  icon: buttonIcon,
                  loading: state.startingGame,
                  onPressed: state.startingGame ? null : action,
                ),
                const SizedBox(height: 13),
                Row(
                  children: [
                    Icon(
                      installed
                          ? Icons.sports_esports_rounded
                          : downloading
                          ? Icons.cloud_sync_rounded
                          : Icons.wifi_rounded,
                      size: 15,
                      color: scheme.onSurfaceVariant.withOpacity(0.82),
                    ),
                    const SizedBox(width: 7),
                    Expanded(
                      child: Text(
                        hint,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: textTheme.labelSmall?.copyWith(
                          color: scheme.onSurfaceVariant.withOpacity(0.82),
                          letterSpacing: 0.1,
                        ),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _PlayScenePainter extends CustomPainter {
  _PlayScenePainter(this.scheme);

  final ColorScheme scheme;

  @override
  void paint(Canvas canvas, Size size) {
    final sunCenter = Offset(size.width * 0.86, size.height * 0.18);
    final sunRect = Rect.fromCircle(
      center: sunCenter,
      radius: size.width * 0.24,
    );
    final sun = Paint()
      ..shader = RadialGradient(
        colors: [scheme.primary.withOpacity(0.16), Colors.transparent],
      ).createShader(sunRect);
    canvas.drawCircle(sunCenter, sunRect.width / 2, sun);

    final random = math.Random(4);
    for (var layer = 0; layer < 3; layer++) {
      final baseline = size.height * (0.74 + layer * 0.09);
      final block = 22.0 + layer * 8;
      final paint = Paint()
        ..color = (layer == 2 ? scheme.surfaceContainer : scheme.primary)
            .withOpacity(layer == 2 ? 0.55 : 0.045);
      final path = Path()..moveTo(0, size.height);
      var x = 0.0;
      while (x < size.width) {
        final height = random.nextDouble() * size.height * 0.07;
        final top = baseline - height;
        path.lineTo(x, top);
        path.lineTo(math.min(x + block, size.width), top);
        x += block;
      }
      path.lineTo(size.width, size.height);
      path.close();
      canvas.drawPath(path, paint);
    }
  }

  @override
  bool shouldRepaint(covariant _PlayScenePainter oldDelegate) {
    return oldDelegate.scheme != scheme;
  }
}

class _StatsRow extends StatelessWidget {
  const _StatsRow({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final strings = context.strings;
    final renderer = switch (state.settings.renderer) {
      'compatibility' => strings.t('rendererShortCompat'),
      'opengl' => strings.t('rendererShortGl'),
      _ => strings.t('rendererShortVulkan'),
    };
    final input = switch (state.settings.input) {
      'touch' => strings.t('inputTouch'),
      'keyboard' => strings.t('inputKeyboard'),
      _ => strings.t('inputAuto'),
    };
    return Row(
      children: [
        Expanded(
          child: StatCard(
            icon: Icons.view_in_ar_rounded,
            label: strings.t('version'),
            value: strings.t('versionNumber'),
          ),
        ),
        const SizedBox(width: 10),
        Expanded(
          child: StatCard(
            icon: Icons.memory_rounded,
            label: strings.t('renderer'),
            value: renderer,
            accent: Theme.of(context).colorScheme.tertiary,
          ),
        ),
        const SizedBox(width: 10),
        Expanded(
          child: StatCard(
            icon: Icons.touch_app_rounded,
            label: strings.t('inputMode'),
            value: input,
            accent: Theme.of(context).colorScheme.secondary,
          ),
        ),
      ],
    );
  }
}

class _Footer extends StatelessWidget {
  const _Footer();

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final strings = context.strings;
    final textTheme = Theme.of(context).textTheme;
    return Column(
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const LogoMark(size: 22),
            const SizedBox(width: 9),
            Text(
              'HORIZON',
              style: textTheme.labelSmall?.copyWith(
                letterSpacing: 3,
                color: scheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
        const SizedBox(height: 10),
        Text(
          strings.t('madeWith'),
          style: textTheme.labelSmall?.copyWith(
            color: scheme.onSurfaceVariant.withOpacity(0.8),
          ),
        ),
      ],
    );
  }
}
