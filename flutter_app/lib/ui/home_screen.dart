import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../core/app_state.dart';
import '../core/app_strings.dart';
import '../core/theme.dart';
import 'download_sheet.dart';
import 'widgets/action_button.dart';
import 'widgets/app_card.dart';
import 'widgets/logo_mark.dart';
import 'widgets/pixel_avatar.dart';
import 'widgets/pixel_backdrop.dart';
import 'widgets/section_header.dart';
import 'widgets/stat_card.dart';
import 'widgets/status_pill.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key, required this.state, required this.onNavigate});

  final AppState state;
  final ValueChanged<int> onNavigate;

  @override
  Widget build(BuildContext context) {
    final strings = context.strings;
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
                key: const PageStorageKey('home-scroll'),
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
                        SectionHeader(
                          title: strings.t('checklist'),
                          subtitle: strings.t('homeSubtitle'),
                        ),
                        _Checklist(state: state),
                        SectionHeader(title: strings.t('quickAccess')),
                        _QuickAccess(
                          state: state,
                          onAccount: () => onNavigate(1),
                          onSettings: () => onNavigate(2),
                        ),
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

    final pillLabel = installed
        ? strings.t('ready')
        : downloading
        ? strings.t('downloading')
        : failed
        ? strings.t('statusFailed')
        : strings.t('notInstalled');
    final pillColor = installed
        ? scheme.primary
        : downloading
        ? scheme.tertiary
        : failed
        ? scheme.error
        : scheme.secondary;

    final summary = installed
        ? strings.t('summaryInstalled')
        : downloading
        ? strings.t('summaryDownloading')
        : failed
        ? strings.t('summaryFailed')
        : strings.t('summaryNotInstalled');

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

    final progress = status?.progress;
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
                Row(
                  children: [
                    StatusPill(label: pillLabel, color: pillColor),
                    const Spacer(),
                    Container(
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
                  ],
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
                const SizedBox(height: 8),
                ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 290),
                  child: Text(
                    summary,
                    style: textTheme.bodyMedium?.copyWith(
                      color: scheme.onSurfaceVariant,
                      height: 1.4,
                    ),
                  ),
                ),
                if (downloading) ...[
                  const SizedBox(height: 22),
                  ClipRRect(
                    borderRadius: BorderRadius.circular(999),
                    child: LinearProgressIndicator(
                      value: progress,
                      minHeight: 7,
                      backgroundColor: scheme.surfaceContainerHighest,
                    ),
                  ),
                  const SizedBox(height: 9),
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          strings.t(status?.stageKey ?? 'preparing'),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: textTheme.labelMedium?.copyWith(
                            color: scheme.onSurfaceVariant,
                          ),
                        ),
                      ),
                      if (progress != null)
                        Text(
                          '${status!.percent}%',
                          style: textTheme.labelLarge?.copyWith(
                            color: scheme.primary,
                          ),
                        ),
                    ],
                  ),
                ],
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
      var index = 0;
      while (x < size.width) {
        final height = random.nextDouble() * size.height * 0.07;
        final top = baseline - height;
        path.lineTo(x, top);
        path.lineTo(math.min(x + block, size.width), top);
        x += block;
        index++;
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

class _Checklist extends StatelessWidget {
  const _Checklist({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final strings = context.strings;
    final installed = state.installed;
    final failed = state.status?.failed ?? false;
    final downloading = state.downloading;
    return AppCard(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Column(
        children: [
          _CheckRow(
            icon: Icons.inventory_2_outlined,
            title: strings.t('resources'),
            subtitle: installed
                ? strings.t('statusInstalled')
                : downloading
                ? strings.t('statusDownloading')
                : failed
                ? strings.t('statusFailed')
                : strings.t('statusNotInstalled'),
            trailing: installed
                ? Icon(
                    Icons.check_circle_rounded,
                    color: scheme.primary,
                    size: 22,
                  )
                : downloading
                ? SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      color: scheme.tertiary,
                    ),
                  )
                : Icon(
                    Icons.arrow_circle_down_rounded,
                    color: scheme.secondary,
                    size: 22,
                  ),
          ),
          Divider(indent: 68, color: scheme.outlineVariant.withOpacity(0.4)),
          _CheckRow(
            icon: Icons.touch_app_outlined,
            title: strings.t('touchControls'),
            subtitle: strings.t('touchControlsHint'),
            trailing: Icon(
              Icons.check_circle_rounded,
              color: scheme.primary,
              size: 22,
            ),
          ),
          Divider(indent: 68, color: scheme.outlineVariant.withOpacity(0.4)),
          _CheckRow(
            icon: Icons.person_outline_rounded,
            title: strings.t('accountCheck'),
            subtitle: '${strings.t('accountOfflineHint')} ${state.displayName}',
            trailing: StatusPill(
              label: strings.t('offline'),
              color: scheme.secondary,
              compact: true,
            ),
          ),
        ],
      ),
    );
  }
}

class _CheckRow extends StatelessWidget {
  const _CheckRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.trailing,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final Widget trailing;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Row(
        children: [
          Container(
            width: 38,
            height: 38,
            decoration: BoxDecoration(
              color: scheme.surfaceContainerHigh.withOpacity(0.8),
              borderRadius: BorderRadius.circular(12),
            ),
            child: Icon(icon, size: 19, color: scheme.onSurfaceVariant),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: textTheme.bodyLarge?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  subtitle,
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
          trailing,
        ],
      ),
    );
  }
}

class _QuickAccess extends StatelessWidget {
  const _QuickAccess({
    required this.state,
    required this.onAccount,
    required this.onSettings,
  });

  final AppState state;
  final VoidCallback onAccount;
  final VoidCallback onSettings;

  @override
  Widget build(BuildContext context) {
    final strings = context.strings;
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Row(
      children: [
        Expanded(
          child: AppCard(
            onTap: onAccount,
            padding: const EdgeInsets.all(16),
            radius: 22,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Container(
                  width: 38,
                  height: 38,
                  decoration: BoxDecoration(
                    color: scheme.secondary.withOpacity(0.13),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Icon(
                    Icons.person_rounded,
                    color: scheme.secondary,
                    size: 20,
                  ),
                ),
                const SizedBox(height: 16),
                Text(
                  strings.t('navAccount'),
                  style: textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 3),
                Text(
                  state.displayName,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(width: 10),
        Expanded(
          child: AppCard(
            onTap: onSettings,
            padding: const EdgeInsets.all(16),
            radius: 22,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Container(
                  width: 38,
                  height: 38,
                  decoration: BoxDecoration(
                    color: scheme.tertiary.withOpacity(0.13),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Icon(
                    Icons.tune_rounded,
                    color: scheme.tertiary,
                    size: 20,
                  ),
                ),
                const SizedBox(height: 16),
                Text(
                  strings.t('navSettings'),
                  style: textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 3),
                Text(
                  strings.t(accentById(state.settings.accent).labelKey),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
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
        const SizedBox(height: 4),
        Text(
          strings.t('disclaimerShort'),
          style: textTheme.labelSmall?.copyWith(
            color: scheme.onSurfaceVariant.withOpacity(0.55),
          ),
        ),
      ],
    );
  }
}
