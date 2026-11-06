import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core/app_state.dart';
import '../core/app_strings.dart';
import 'widgets/app_card.dart';
import 'widgets/horizon_toast.dart';
import 'widgets/pixel_avatar.dart';
import 'widgets/pixel_backdrop.dart';
import 'widgets/section_header.dart';
import 'widgets/status_pill.dart';

class AccountScreen extends StatefulWidget {
  const AccountScreen({super.key, required this.state});

  final AppState state;

  @override
  State<AccountScreen> createState() => _AccountScreenState();
}

class _AccountScreenState extends State<AccountScreen> {
  late final TextEditingController _nameController;
  late final TextEditingController _serverController;

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController(text: widget.state.settings.name);
    _serverController = TextEditingController(
      text: widget.state.settings.server,
    );
  }

  @override
  void dispose() {
    _nameController.dispose();
    _serverController.dispose();
    super.dispose();
  }

  Future<void> _saveName() async {
    final value = _nameController.text.trim();
    if (value.isEmpty) {
      _snack(context.strings.t('nameRequired'), error: true);
      return;
    }
    if (value.length > 16) {
      _snack(context.strings.t('nameTooLong'), error: true);
      return;
    }
    await widget.state.setName(value);
    if (!mounted) {
      return;
    }
    FocusScope.of(context).unfocus();
    _snack(context.strings.t('profileSaved'));
  }

  Future<void> _saveServer() async {
    await widget.state.setServer(_serverController.text.trim());
    if (!mounted) {
      return;
    }
    FocusScope.of(context).unfocus();
    _snack(context.strings.t('serverSaved'));
  }

  Future<void> _clearServer() async {
    _serverController.clear();
    await widget.state.setServer('');
    if (!mounted) {
      return;
    }
    setState(() {});
    _snack(context.strings.t('serverCleared'));
  }

  Future<void> _pickSkin() async {
    final saved = await widget.state.pickSkin();
    if (!mounted) {
      return;
    }
    if (saved) {
      setState(() {});
      _snack(context.strings.t('skinSaved'));
    } else if (widget.state.errorMessage == 'invalid-skin') {
      _snack(context.strings.t('invalidSkin'), error: true);
    }
    widget.state.clearError();
  }

  Future<void> _removeSkin() async {
    await widget.state.clearSkin();
    if (!mounted) {
      return;
    }
    setState(() {});
    _snack(context.strings.t('skinRemoved'));
  }

  void _snack(String message, {bool error = false}) {
    showHorizonToast(
      context,
      message,
      icon: error ? Icons.error_outline_rounded : Icons.check_circle_rounded,
    );
  }

  @override
  Widget build(BuildContext context) {
    final state = widget.state;
    final strings = context.strings;
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Scaffold(
      body: Stack(
        children: [
          const PixelBackdrop(),
          SafeArea(
            bottom: false,
            child: ListView(
              key: const PageStorageKey('account-scroll'),
              physics: const BouncingScrollPhysics(
                parent: AlwaysScrollableScrollPhysics(),
              ),
              padding: const EdgeInsets.fromLTRB(20, 22, 20, 128),
              children: [
                Text(
                  strings.t('accountTitle'),
                  style: textTheme.headlineMedium,
                ),
                const SizedBox(height: 6),
                Text(
                  strings.t('accountSubtitle'),
                  style: textTheme.bodyMedium?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 24),
                AppCard(
                  padding: const EdgeInsets.all(20),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          _SkinPreview(
                            name: _nameController.text.isEmpty
                                ? 'Steve'
                                : _nameController.text,
                            skinPath: state.settings.skinPath,
                            slim: state.settings.skinSlim,
                          ),
                          const SizedBox(width: 16),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  _nameController.text.trim().isEmpty
                                      ? strings.t('offlineNameEmpty')
                                      : _nameController.text.trim(),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: textTheme.titleLarge?.copyWith(
                                    fontWeight: FontWeight.w800,
                                    letterSpacing: -0.3,
                                  ),
                                ),
                                const SizedBox(height: 7),
                                StatusPill(
                                  label: strings.t('offline'),
                                  color: scheme.secondary,
                                  compact: true,
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 22),
                      Text(
                        strings.t('offlineProfile'),
                        style: textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 10),
                      TextField(
                        controller: _nameController,
                        onChanged: (_) => setState(() {}),
                        onSubmitted: (_) => _saveName(),
                        textInputAction: TextInputAction.done,
                        inputFormatters: [LengthLimitingTextInputFormatter(16)],
                        decoration: InputDecoration(
                          hintText: strings.t('playerName'),
                          prefixIcon: const Icon(Icons.badge_outlined),
                          helperText: strings.t('offlineProfileHint'),
                        ),
                      ),
                      const SizedBox(height: 14),
                      Row(
                        children: [
                          Expanded(
                            child: FilledButton.tonalIcon(
                              onPressed: _saveName,
                              icon: const Icon(Icons.check_rounded, size: 19),
                              label: Text(strings.t('save')),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 22),
                      Divider(color: scheme.outlineVariant.withOpacity(0.45)),
                      const SizedBox(height: 18),
                      Row(
                        children: [
                          Icon(
                            Icons.face_rounded,
                            size: 19,
                            color: scheme.onSurfaceVariant,
                          ),
                          const SizedBox(width: 9),
                          Expanded(
                            child: Text(
                              strings.t('skinAppearance'),
                              style: textTheme.titleSmall?.copyWith(
                                fontWeight: FontWeight.w700,
                              ),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 6),
                      Text(
                        strings.t('skinHint'),
                        style: textTheme.bodySmall?.copyWith(
                          color: scheme.onSurfaceVariant,
                        ),
                      ),
                      const SizedBox(height: 12),
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: [
                          FilledButton.tonalIcon(
                            onPressed: _pickSkin,
                            icon: const Icon(
                              Icons.add_photo_alternate_outlined,
                              size: 19,
                            ),
                            label: Text(strings.t('chooseSkin')),
                          ),
                          if (state.hasSkin)
                            OutlinedButton.icon(
                              onPressed: _removeSkin,
                              icon: const Icon(
                                Icons.delete_outline_rounded,
                                size: 18,
                              ),
                              label: Text(strings.t('removeSkin')),
                            ),
                        ],
                      ),
                      const SizedBox(height: 12),
                      SegmentedButton<bool>(
                        segments: [
                          ButtonSegment(
                            value: false,
                            icon: const Icon(Icons.person_outline_rounded),
                            label: Text(strings.t('classicArms')),
                          ),
                          ButtonSegment(
                            value: true,
                            icon: const Icon(Icons.accessibility_new),
                            label: Text(strings.t('slimArms')),
                          ),
                        ],
                        selected: {state.settings.skinSlim},
                        showSelectedIcon: false,
                        onSelectionChanged: (value) {
                          state.setSkinSlim(value.first);
                        },
                      ),
                    ],
                  ),
                ),
                SectionHeader(title: strings.t('quickServer')),
                AppCard(
                  padding: const EdgeInsets.all(18),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Container(
                            width: 44,
                            height: 44,
                            decoration: BoxDecoration(
                              color: scheme.tertiary.withOpacity(0.14),
                              borderRadius: BorderRadius.circular(14),
                            ),
                            child: Icon(
                              Icons.dns_outlined,
                              color: scheme.tertiary,
                              size: 22,
                            ),
                          ),
                          const SizedBox(width: 13),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  state.settings.server.trim().isEmpty
                                      ? strings.t('quickServerNone')
                                      : state.settings.server.trim(),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: textTheme.titleSmall?.copyWith(
                                    fontWeight: FontWeight.w700,
                                  ),
                                ),
                                const SizedBox(height: 3),
                                Text(
                                  strings.t('quickServerHint'),
                                  maxLines: 2,
                                  style: textTheme.bodySmall?.copyWith(
                                    color: scheme.onSurfaceVariant,
                                    height: 1.35,
                                  ),
                                ),
                              ],
                            ),
                          ),
                          const SizedBox(width: 8),
                          StatusPill(
                            label: state.settings.server.trim().isEmpty
                                ? strings.t('quickServerNone')
                                : strings.t('quickServerActive'),
                            color: state.settings.server.trim().isEmpty
                                ? scheme.secondary
                                : scheme.primary,
                            compact: true,
                          ),
                        ],
                      ),
                      const SizedBox(height: 16),
                      TextField(
                        controller: _serverController,
                        keyboardType: TextInputType.url,
                        autocorrect: false,
                        enableSuggestions: false,
                        textInputAction: TextInputAction.done,
                        onChanged: (_) => setState(() {}),
                        onSubmitted: (_) => _saveServer(),
                        decoration: InputDecoration(
                          hintText: 'play.example.net',
                          prefixIcon: const Icon(Icons.public),
                          helperText: strings.t('serverAddress'),
                          suffixIcon: _serverController.text.isEmpty
                              ? null
                              : IconButton(
                                  tooltip: strings.t('clear'),
                                  onPressed: _clearServer,
                                  icon: const Icon(
                                    Icons.close_rounded,
                                    size: 19,
                                  ),
                                ),
                        ),
                      ),
                      const SizedBox(height: 14),
                      SizedBox(
                        width: double.infinity,
                        child: FilledButton.icon(
                          onPressed: _saveServer,
                          icon: const Icon(Icons.check_rounded, size: 19),
                          label: Text(strings.t('saveServer')),
                        ),
                      ),
                    ],
                  ),
                ),
                SectionHeader(title: strings.t('microsoftAccount')),
                AppCard(
                  padding: const EdgeInsets.all(20),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Container(
                            width: 44,
                            height: 44,
                            decoration: BoxDecoration(
                              color: scheme.surfaceContainerHigh,
                              borderRadius: BorderRadius.circular(14),
                            ),
                            child: Icon(
                              Icons.lock_outline_rounded,
                              color: scheme.onSurfaceVariant,
                              size: 22,
                            ),
                          ),
                          const SizedBox(width: 14),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  strings.t('microsoftAccount'),
                                  style: textTheme.titleMedium?.copyWith(
                                    fontWeight: FontWeight.w700,
                                  ),
                                ),
                                const SizedBox(height: 3),
                                Text(
                                  strings.t('comingSoon'),
                                  style: textTheme.labelSmall?.copyWith(
                                    color: scheme.onSurfaceVariant,
                                    letterSpacing: 0.4,
                                  ),
                                ),
                              ],
                            ),
                          ),
                          StatusPill(
                            label: strings.t('comingSoon'),
                            color: scheme.secondary,
                            compact: true,
                          ),
                        ],
                      ),
                      const SizedBox(height: 16),
                      Text(
                        strings.t('microsoftUnavailable'),
                        style: textTheme.bodyMedium?.copyWith(
                          color: scheme.onSurface,
                          fontWeight: FontWeight.w600,
                          height: 1.4,
                        ),
                      ),
                      const SizedBox(height: 6),
                      Text(
                        strings.t('microsoftUnavailableHint'),
                        style: textTheme.bodySmall?.copyWith(
                          color: scheme.onSurfaceVariant,
                          height: 1.45,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 24),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _SkinPreview extends StatelessWidget {
  const _SkinPreview({
    required this.name,
    required this.skinPath,
    required this.slim,
  });

  final String name;
  final String skinPath;
  final bool slim;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final file = skinPath.trim().isEmpty ? null : File(skinPath);
    final image = file != null && file.existsSync()
        ? Image.file(
            file,
            width: 74,
            height: 74,
            fit: BoxFit.cover,
            filterQuality: FilterQuality.none,
            errorBuilder: (_, __, ___) => _fallback(scheme),
          )
        : null;
    return Container(
      width: 74,
      height: 74,
      decoration: BoxDecoration(
        color: scheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(22),
        border: Border.all(color: scheme.outlineVariant.withOpacity(0.55)),
      ),
      clipBehavior: Clip.antiAlias,
      child: Stack(
        fit: StackFit.expand,
        children: [
          if (image != null) image else _fallback(scheme),
          Positioned(
            right: 4,
            bottom: 4,
            child: Container(
              width: 18,
              height: 18,
              decoration: BoxDecoration(
                color: scheme.surface.withOpacity(0.88),
                shape: BoxShape.circle,
              ),
              child: Icon(
                slim ? Icons.accessibility_new : Icons.accessibility_new,
                size: 11,
                color: scheme.onSurfaceVariant,
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _fallback(ColorScheme scheme) {
    return Center(child: PixelAvatar(name: name, size: 58));
  }
}
