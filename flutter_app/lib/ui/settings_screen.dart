import 'package:flutter/material.dart';

import '../core/app_state.dart';
import '../core/app_strings.dart';
import '../core/theme.dart';
import 'widgets/horizon_toast.dart';
import 'widgets/pixel_backdrop.dart';
import 'widgets/section_header.dart';
import 'widgets/settings_tile.dart';

const String _githubUrl = 'https://github.com/devnusret/rusttest';
const String _discordUrl = 'https://discord.gg/PG79z2PhjG';

class SettingsScreen extends StatelessWidget {
  const SettingsScreen({super.key, required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final strings = context.strings;
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final settings = state.settings;
    final accent = accentById(settings.accent);

    return Scaffold(
      body: Stack(
        children: [
          const PixelBackdrop(),
          SafeArea(
            bottom: false,
            child: ListView(
              key: const PageStorageKey('settings-scroll'),
              physics: const BouncingScrollPhysics(
                parent: AlwaysScrollableScrollPhysics(),
              ),
              padding: const EdgeInsets.fromLTRB(20, 22, 20, 128),
              children: [
                Text(
                  strings.t('settingsTitle'),
                  style: textTheme.headlineMedium,
                ),
                const SizedBox(height: 6),
                Text(
                  strings.t('settingsSubtitle'),
                  style: textTheme.bodyMedium?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
                SectionHeader(title: strings.t('settingsGroupLook')),
                SettingsGroup(
                  children: [
                    SettingsTile(
                      icon: Icons.language_rounded,
                      title: strings.t('language'),
                      value: switch (settings.locale) {
                        'tr' => strings.t('turkish'),
                        'en' => strings.t('english'),
                        _ => strings.t('system'),
                      },
                      onTap: () {
                        _chooseLanguage(context, state);
                      },
                    ),
                    SettingsTile(
                      icon: Icons.dark_mode_outlined,
                      title: strings.t('theme'),
                      value: switch (settings.theme) {
                        'dark' => strings.t('dark'),
                        'light' => strings.t('light'),
                        _ => strings.t('system'),
                      },
                      onTap: () {
                        _chooseTheme(context, state);
                      },
                    ),
                    SettingsTile(
                      icon: Icons.palette_outlined,
                      title: strings.t('accent'),
                      value: strings.t(accent.labelKey),
                      onTap: () {
                        _chooseAccent(context, state);
                      },
                    ),
                  ],
                ),
                SectionHeader(title: strings.t('settingsGroupGame')),
                SettingsGroup(
                  children: [
                    SettingsTile(
                      icon: Icons.memory_rounded,
                      title: strings.t('renderer'),
                      subtitle: switch (settings.renderer) {
                        'compatibility' => strings.t('rendererCompatHint'),
                        'opengl' => strings.t('rendererGlHint'),
                        _ => strings.t('rendererVulkanHint'),
                      },
                      value: switch (settings.renderer) {
                        'compatibility' => strings.t('rendererShortCompat'),
                        'opengl' => strings.t('rendererShortGl'),
                        _ => strings.t('rendererShortVulkan'),
                      },
                      onTap: () {
                        _chooseRenderer(context, state);
                      },
                    ),
                    SettingsTile(
                      icon: Icons.screen_rotation_rounded,
                      title: strings.t('gameOrientation'),
                      subtitle: strings.t('gameOrientationHint'),
                      value: switch (settings.orientation) {
                        'portrait' => strings.t('portrait'),
                        'auto' => strings.t('automatic'),
                        _ => strings.t('landscape'),
                      },
                      onTap: () {
                        _chooseOrientation(context, state);
                      },
                    ),
                    SettingsTile(
                      icon: Icons.keyboard_rounded,
                      title: strings.t('inputMode'),
                      subtitle: strings.t('inputAutoHint'),
                      value: switch (settings.input) {
                        'touch' => strings.t('inputTouch'),
                        'keyboard' => strings.t('inputKeyboard'),
                        _ => strings.t('inputAuto'),
                      },
                      onTap: () {
                        _chooseInput(context, state);
                      },
                    ),
                    SettingsTile(
                      icon: Icons.sports_esports_outlined,
                      title: strings.t('editControls'),
                      subtitle: strings.t('editControlsHint'),
                      onTap: () {
                        _openControlEditor(context, state);
                      },
                    ),
                  ],
                ),
                SectionHeader(title: strings.t('settingsGroupDownload')),
                SettingsGroup(
                  children: [
                    SwitchSettingsTile(
                      icon: Icons.cloud_download_outlined,
                      title: strings.t('backgroundDownloads'),
                      subtitle: strings.t('backgroundDownloadsHint'),
                      value: settings.backgroundDownloads,
                      onChanged: (value) {
                        state.setBackgroundDownloads(value);
                        _snack(context, strings.t('settingsSaved'));
                      },
                    ),
                    SettingsTile(
                      icon: Icons.notifications_none_rounded,
                      title: strings.t('updateNotifications'),
                      subtitle: strings.t('notificationPermission'),
                      onTap: () {
                        state.requestNotifications();
                      },
                    ),
                  ],
                ),
                SectionHeader(title: strings.t('settingsGroupAbout')),
                SettingsGroup(
                  children: [
                    SettingsTile(
                      icon: Icons.info_outline_rounded,
                      title: strings.t('appVersion'),
                      value: state.bridge.versionName,
                    ),
                    SettingsTile(
                      icon: Icons.sports_esports_outlined,
                      title: strings.t('gameVersion'),
                      value: AppState.gameVersion,
                    ),
                    SettingsTile(
                      icon: Icons.description_outlined,
                      title: strings.t('licenses'),
                      onTap: () {
                        showLicensePage(
                          context: context,
                          applicationName: 'Horizon',
                          applicationVersion: state.bridge.versionName,
                        );
                      },
                    ),
                    SettingsTile(
                      icon: Icons.code_rounded,
                      title: strings.t('github'),
                      onTap: () {
                        state.openUrl(_githubUrl);
                      },
                    ),
                    SettingsTile(
                      icon: Icons.forum_outlined,
                      title: strings.t('discord'),
                      onTap: () {
                        state.openUrl(_discordUrl);
                      },
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

  void _snack(BuildContext context, String message) {
    showHorizonToast(context, message);
  }
}

Future<void> _openControlEditor(BuildContext context, AppState state) async {
  final layout = await state.loadTouchLayout();
  if (!context.mounted) {
    return;
  }
  await Navigator.of(context).push(
    MaterialPageRoute<void>(
      builder: (_) => ControlEditorScreen(state: state, layout: layout),
    ),
  );
}

Future<void> _chooseLanguage(BuildContext context, AppState state) {
  final strings = context.strings;
  return _choose(
    context,
    title: strings.t('language'),
    selected: state.settings.locale,
    choices: [
      _Choice('system', strings.t('system'), icon: Icons.settings_outlined),
      _Choice('tr', strings.t('turkish'), icon: Icons.translate_rounded),
      _Choice('en', strings.t('english'), icon: Icons.translate_rounded),
    ],
    onSelected: (value) {
      state.setLocale(value);
    },
  );
}

Future<void> _chooseTheme(BuildContext context, AppState state) {
  final strings = context.strings;
  return _choose(
    context,
    title: strings.t('theme'),
    selected: state.settings.theme,
    choices: [
      _Choice(
        'system',
        strings.t('system'),
        icon: Icons.brightness_auto_outlined,
      ),
      _Choice('dark', strings.t('dark'), icon: Icons.dark_mode_outlined),
      _Choice('light', strings.t('light'), icon: Icons.light_mode_outlined),
    ],
    onSelected: (value) {
      state.setTheme(value);
    },
  );
}

Future<void> _chooseRenderer(BuildContext context, AppState state) {
  final strings = context.strings;
  return _choose(
    context,
    title: strings.t('renderer'),
    selected: state.settings.renderer,
    choices: [
      _Choice(
        'vulkan',
        strings.t('rendererVulkan'),
        subtitle: strings.t('rendererVulkanHint'),
        icon: Icons.bolt_rounded,
      ),
      _Choice(
        'compatibility',
        strings.t('rendererCompat'),
        subtitle: strings.t('rendererCompatHint'),
        icon: Icons.shield_outlined,
      ),
      _Choice(
        'opengl',
        strings.t('rendererGl'),
        subtitle: strings.t('rendererGlHint'),
        icon: Icons.science_outlined,
      ),
    ],
    onSelected: (value) {
      state.setRenderer(value);
    },
  );
}

Future<void> _chooseOrientation(BuildContext context, AppState state) {
  final strings = context.strings;
  return _choose(
    context,
    title: strings.t('gameOrientation'),
    selected: state.settings.orientation,
    choices: [
      _Choice(
        'landscape',
        strings.t('landscape'),
        icon: Icons.stay_current_landscape_rounded,
      ),
      _Choice(
        'portrait',
        strings.t('portrait'),
        icon: Icons.stay_current_portrait_rounded,
      ),
      _Choice(
        'auto',
        strings.t('automatic'),
        icon: Icons.screen_rotation_rounded,
      ),
    ],
    onSelected: (value) {
      state.setOrientation(value);
    },
  );
}

Future<void> _chooseInput(BuildContext context, AppState state) {
  final strings = context.strings;
  return _choose(
    context,
    title: strings.t('inputMode'),
    selected: state.settings.input,
    choices: [
      _Choice(
        'auto',
        strings.t('inputAuto'),
        icon: Icons.auto_awesome_outlined,
      ),
      _Choice('touch', strings.t('inputTouch'), icon: Icons.touch_app_outlined),
      _Choice(
        'keyboard',
        strings.t('inputKeyboard'),
        icon: Icons.keyboard_outlined,
      ),
    ],
    onSelected: (value) {
      state.setInput(value);
    },
  );
}

Future<void> _chooseAccent(BuildContext context, AppState state) {
  final strings = context.strings;
  final scheme = Theme.of(context).colorScheme;
  return showModalBottomSheet<String>(
    context: context,
    showDragHandle: true,
    isScrollControlled: true,
    backgroundColor: scheme.surfaceContainerLow,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(
        top: Radius.circular(HorizonRadius.sheet),
      ),
    ),
    builder: (sheetContext) {
      return SafeArea(
        top: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(22, 0, 22, 28),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                strings.t('accent'),
                style: Theme.of(sheetContext).textTheme.titleLarge
                    ?.copyWith(fontWeight: FontWeight.w800),
              ),
              const SizedBox(height: 20),
              Wrap(
                spacing: 18,
                runSpacing: 18,
                children: accents.map((accent) {
                  final selected = state.settings.accent == accent.id;
                  return InkWell(
                    borderRadius: BorderRadius.circular(20),
                    onTap: () => Navigator.of(sheetContext).pop(accent.id),
                    child: SizedBox(
                      width: 86,
                      child: Column(
                        children: [
                          AnimatedContainer(
                            duration: const Duration(milliseconds: 220),
                            width: 58,
                            height: 58,
                            decoration: BoxDecoration(
                              color: accent.seed,
                              shape: BoxShape.circle,
                              border: Border.all(
                                color: selected
                                    ? Theme.of(sheetContext)
                                          .colorScheme
                                          .onSurface
                                    : Colors.transparent,
                                width: 2.5,
                              ),
                              boxShadow: [
                                BoxShadow(
                                  color: accent.seed.withOpacity(0.32),
                                  blurRadius: 16,
                                  offset: const Offset(0, 6),
                                ),
                              ],
                            ),
                            child: selected
                                ? const Icon(
                                    Icons.check_rounded,
                                    color: Colors.white,
                                    size: 26,
                                  )
                                : null,
                          ),
                          const SizedBox(height: 9),
                          Text(
                            strings.t(accent.labelKey),
                            textAlign: TextAlign.center,
                            style: Theme.of(sheetContext).textTheme.labelMedium
                                ?.copyWith(
                                  fontWeight: selected
                                      ? FontWeight.w800
                                      : FontWeight.w600,
                                ),
                          ),
                        ],
                      ),
                    ),
                  );
                }).toList(),
              ),
            ],
          ),
        ),
      );
    },
  ).then((value) {
    if (value != null) {
      state.setAccent(value);
    }
  });
}

class _Choice {
  const _Choice(this.value, this.label, {this.subtitle, this.icon});

  final String value;
  final String label;
  final String? subtitle;
  final IconData? icon;
}

Future<void> _choose(
  BuildContext context, {
  required String title,
  required String selected,
  required List<_Choice> choices,
  required ValueChanged<String> onSelected,
}) {
  final scheme = Theme.of(context).colorScheme;
  return showModalBottomSheet<String>(
    context: context,
    showDragHandle: true,
    isScrollControlled: true,
    backgroundColor: scheme.surfaceContainerLow,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(
        top: Radius.circular(HorizonRadius.sheet),
      ),
    ),
    builder: (sheetContext) {
      return SafeArea(
        top: false,
        child: SingleChildScrollView(
          padding: const EdgeInsets.fromLTRB(18, 0, 18, 24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(4, 0, 4, 12),
                child: Text(
                  title,
                  style: Theme.of(sheetContext).textTheme.titleLarge
                      ?.copyWith(fontWeight: FontWeight.w800),
                ),
              ),
              ...choices.map((choice) {
                final isSelected = choice.value == selected;
                return Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: Material(
                    color: isSelected
                        ? scheme.primary.withOpacity(0.1)
                        : scheme.surfaceContainerHigh.withOpacity(0.55),
                    borderRadius: BorderRadius.circular(18),
                    child: InkWell(
                      borderRadius: BorderRadius.circular(18),
                      onTap: () => Navigator.of(sheetContext).pop(choice.value),
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 14,
                          vertical: 13,
                        ),
                        child: Row(
                          children: [
                            if (choice.icon != null)
                              Container(
                                width: 38,
                                height: 38,
                                decoration: BoxDecoration(
                                  color: scheme.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(12),
                                ),
                                child: Icon(
                                  choice.icon,
                                  size: 19,
                                  color: isSelected
                                      ? scheme.primary
                                      : scheme.onSurfaceVariant,
                                ),
                              ),
                            if (choice.icon != null) const SizedBox(width: 13),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    choice.label,
                                    style: Theme.of(sheetContext)
                                        .textTheme
                                        .bodyLarge
                                        ?.copyWith(fontWeight: FontWeight.w600),
                                  ),
                                  if (choice.subtitle != null) ...[
                                    const SizedBox(height: 3),
                                    Text(
                                      choice.subtitle!,
                                      style: Theme.of(sheetContext)
                                          .textTheme
                                          .bodySmall
                                          ?.copyWith(
                                            color: scheme.onSurfaceVariant,
                                          ),
                                    ),
                                  ],
                                ],
                              ),
                            ),
                            Icon(
                              isSelected
                                  ? Icons.radio_button_checked_rounded
                                  : Icons.radio_button_unchecked_rounded,
                              size: 21,
                              color: isSelected
                                  ? scheme.primary
                                  : scheme.onSurfaceVariant,
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                );
              }),
            ],
          ),
        ),
      );
    },
  ).then((value) {
    if (value != null) {
      onSelected(value);
    }
  });
}
