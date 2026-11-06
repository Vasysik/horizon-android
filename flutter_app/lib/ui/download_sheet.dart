import 'package:flutter/material.dart';

import '../core/app_state.dart';
import '../core/app_strings.dart';
import '../core/format.dart';
import '../core/install_status.dart';
import '../core/theme.dart';
import 'widgets/action_button.dart';
import 'widgets/status_pill.dart';

Future<void> showDownloadSheet(BuildContext context, AppState state) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    backgroundColor: Colors.transparent,
    builder: (context) => DownloadSheet(state: state),
  );
}

class DownloadSheet extends StatelessWidget {
  const DownloadSheet({super.key, required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final strings = context.strings;
    return AnimatedBuilder(
      animation: state,
      builder: (context, _) {
        final status = state.status;
        final installed = state.installed;
        final running = status?.running ?? false;
        final failed = status?.failed ?? false;
        final completed = installed || (status?.completed ?? false);
        final progress = status?.progress;
        final stageLabel = strings.t(status?.stageKey ?? 'preparing');

        return Container(
          constraints: BoxConstraints(
            maxHeight: MediaQuery.sizeOf(context).height * 0.9,
          ),
          decoration: BoxDecoration(
            color: scheme.surfaceContainerLow,
            borderRadius: const BorderRadius.vertical(
              top: Radius.circular(HorizonRadius.sheet),
            ),
            border: Border.all(color: scheme.outlineVariant.withOpacity(0.55)),
          ),
          child: SingleChildScrollView(
            padding: const EdgeInsets.fromLTRB(22, 10, 22, 26),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Center(
                  child: Container(
                    width: 36,
                    height: 4,
                    decoration: BoxDecoration(
                      color: scheme.outlineVariant,
                      borderRadius: BorderRadius.circular(999),
                    ),
                  ),
                ),
                const SizedBox(height: 18),
                Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            strings.t('downloadSheetTitle'),
                            style: Theme.of(context).textTheme.titleLarge
                                ?.copyWith(
                                  fontWeight: FontWeight.w800,
                                  letterSpacing: -0.3,
                                ),
                          ),
                          const SizedBox(height: 3),
                          Text(
                            strings.t('downloadSheetSubtitle'),
                            style: Theme.of(context).textTheme.bodySmall
                                ?.copyWith(color: scheme.onSurfaceVariant),
                          ),
                        ],
                      ),
                    ),
                    IconButton(
                      onPressed: () => Navigator.of(context).pop(),
                      icon: const Icon(Icons.close_rounded),
                    ),
                  ],
                ),
                const SizedBox(height: 18),
                Center(
                  child: _ProgressOrb(
                    progress: progress,
                    running: running,
                    completed: completed,
                    failed: failed,
                    percent: status?.percent ?? 0,
                    label: completed
                        ? strings.t('downloadComplete')
                        : failed
                        ? strings.t('statusFailed')
                        : stageLabel,
                  ),
                ),
                const SizedBox(height: 18),
                Center(
                  child: StatusPill(
                    label: completed
                        ? strings.t('ready')
                        : failed
                        ? strings.t('statusFailed')
                        : running
                        ? strings.t('downloading')
                        : strings.t('notInstalled'),
                    color: completed
                        ? scheme.primary
                        : failed
                        ? scheme.error
                        : running
                        ? scheme.tertiary
                        : scheme.secondary,
                  ),
                ),
                const SizedBox(height: 22),
                if (running && status?.stageId == 1 && status!.total > 0)
                  _Metrics(
                    leftLabel: strings.t('storage'),
                    leftValue:
                        '${formatBytes(status!.done)} / ${formatBytes(status.total)}',
                    middleLabel: strings.t('speed'),
                    middleValue: formatSpeed(state.speed),
                    rightLabel: strings.t('remaining'),
                    rightValue: state.eta == null
                        ? strings.t('etaUnknown')
                        : formatDuration(state.eta),
                  )
                else if (running && status?.stageId == 4 && status!.total > 0)
                  _Metrics(
                    leftLabel: strings.t('stageControls'),
                    leftValue:
                        '${status!.done} / ${status.total} ${strings.t('files')}',
                    middleLabel: strings.t('speed'),
                    middleValue: '—',
                    rightLabel: strings.t('remaining'),
                    rightValue: '—',
                  ),
                if (running) const SizedBox(height: 20),
                _StageList(
                  statusId: status?.stageId ?? 0,
                  completed: completed,
                  running: running,
                ),
                if (failed) ...[
                  const SizedBox(height: 20),
                  Container(
                    padding: const EdgeInsets.all(14),
                    decoration: BoxDecoration(
                      color: scheme.errorContainer.withOpacity(0.55),
                      borderRadius: BorderRadius.circular(16),
                      border: Border.all(color: scheme.error.withOpacity(0.25)),
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Icon(
                              Icons.error_outline_rounded,
                              color: scheme.error,
                              size: 19,
                            ),
                            const SizedBox(width: 9),
                            Expanded(
                              child: Text(
                                strings.t('downloadFailed'),
                                style: Theme.of(context).textTheme.titleSmall
                                    ?.copyWith(
                                      color: scheme.onErrorContainer,
                                      fontWeight: FontWeight.w700,
                                    ),
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 8),
                        Text(
                          strings.t('downloadFailedHint'),
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(
                                color: scheme.onErrorContainer,
                                height: 1.4,
                              ),
                        ),
                        if (status?.error.isNotEmpty ?? false) ...[
                          const SizedBox(height: 8),
                          Text(
                            status!.error,
                            maxLines: 5,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.labelSmall
                                ?.copyWith(
                                  color: scheme.onErrorContainer.withOpacity(
                                    0.75,
                                  ),
                                ),
                          ),
                        ],
                      ],
                    ),
                  ),
                ],
                if (state.stalled && running && !failed) ...[
                  const SizedBox(height: 16),
                  ActionButton(
                    label: strings.t('resume'),
                    icon: Icons.refresh_rounded,
                    tone: ActionTone.tonal,
                    onPressed: () {
                      state.resumeDownload();
                    },
                  ),
                ],
                const SizedBox(height: 22),
                if (completed)
                  ActionButton(
                    label: strings.t('openGame'),
                    icon: Icons.play_arrow_rounded,
                    loading: state.startingGame,
                    onPressed: () {
                      state.launchGame();
                      Navigator.of(context).pop();
                    },
                  )
                else if (failed)
                  ActionButton(
                    label: strings.t('retry'),
                    icon: Icons.refresh_rounded,
                    onPressed: () {
                      state.startDownload();
                    },
                  )
                else
                  ActionButton(
                    label: strings.t('backgroundDownloads'),
                    icon: Icons.cloud_sync_rounded,
                    tone: ActionTone.tonal,
                    onPressed: () {
                      state.resumeDownload();
                    },
                  ),
                const SizedBox(height: 14),
                Text(
                  strings.t('freeSpaceNote'),
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                    color: scheme.onSurfaceVariant.withOpacity(0.7),
                    height: 1.4,
                    letterSpacing: 0.1,
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _ProgressOrb extends StatelessWidget {
  const _ProgressOrb({
    required this.progress,
    required this.running,
    required this.completed,
    required this.failed,
    required this.percent,
    required this.label,
  });

  final double? progress;
  final bool running;
  final bool completed;
  final bool failed;
  final int percent;
  final String label;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final color = failed
        ? scheme.error
        : completed
        ? scheme.primary
        : scheme.tertiary;
    final value = completed ? 1.0 : progress;
    return SizedBox(
      width: 154,
      height: 154,
      child: Stack(
        alignment: Alignment.center,
        children: [
          SizedBox(
            width: 154,
            height: 154,
            child: CircularProgressIndicator(
              value: value,
              strokeWidth: 9,
              strokeCap: StrokeCap.round,
              color: color,
              backgroundColor: scheme.surfaceContainerHighest,
            ),
          ),
          Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (completed)
                Icon(Icons.check_rounded, size: 40, color: color)
              else
                Text(
                  running && value != null ? '$percent%' : '…',
                  style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                    fontWeight: FontWeight.w800,
                    color: scheme.onSurface,
                    letterSpacing: -1,
                  ),
                ),
              const SizedBox(height: 4),
              SizedBox(
                width: 112,
                child: Text(
                  label,
                  textAlign: TextAlign.center,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                    letterSpacing: 0.1,
                    height: 1.25,
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _Metrics extends StatelessWidget {
  const _Metrics({
    required this.leftLabel,
    required this.leftValue,
    required this.middleLabel,
    required this.middleValue,
    required this.rightLabel,
    required this.rightValue,
  });

  final String leftLabel;
  final String leftValue;
  final String middleLabel;
  final String middleValue;
  final String rightLabel;
  final String rightValue;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    Widget item(String label, String value) {
      return Expanded(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textTheme.labelSmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              value,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textTheme.labelLarge?.copyWith(color: scheme.onSurface),
            ),
          ],
        ),
      );
    }

    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: scheme.surfaceContainer.withOpacity(0.7),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: scheme.outlineVariant.withOpacity(0.45)),
      ),
      child: Row(
        children: [
          item(leftLabel, leftValue),
          const SizedBox(width: 10),
          item(middleLabel, middleValue),
          const SizedBox(width: 10),
          item(rightLabel, rightValue),
        ],
      ),
    );
  }
}

class _StageList extends StatelessWidget {
  const _StageList({
    required this.statusId,
    required this.completed,
    required this.running,
  });

  final int statusId;
  final bool completed;
  final bool running;

  static const List<String> _keys = [
    'stageMetadata',
    'stageDownload',
    'stageExtract',
    'stageWorldData',
    'stageControls',
    'stageFinalize',
  ];

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final strings = context.strings;
    final textTheme = Theme.of(context).textTheme;
    return Column(
      children: List.generate(_keys.length, (index) {
        final done = completed || statusId > index;
        final active = !completed && running && statusId == index;
        final Color color;
        if (done) {
          color = scheme.primary;
        } else if (active) {
          color = scheme.tertiary;
        } else {
          color = scheme.onSurfaceVariant.withOpacity(0.4);
        }
        return Padding(
          padding: const EdgeInsets.symmetric(vertical: 5),
          child: Row(
            children: [
              SizedBox(
                width: 24,
                height: 24,
                child: done
                    ? Icon(Icons.check_circle_rounded, size: 20, color: color)
                    : active
                    ? SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          color: scheme.tertiary,
                        ),
                      )
                    : Container(
                        width: 8,
                        height: 8,
                        margin: const EdgeInsets.only(left: 6),
                        decoration: BoxDecoration(
                          color: color,
                          shape: BoxShape.circle,
                        ),
                      ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  strings.t(_keys[index]),
                  style: textTheme.bodySmall?.copyWith(
                    color: done || active
                        ? scheme.onSurface
                        : scheme.onSurfaceVariant,
                    fontWeight: active ? FontWeight.w700 : FontWeight.w500,
                  ),
                ),
              ),
            ],
          ),
        );
      }),
    );
  }
}
