import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core/app_state.dart';
import '../core/app_strings.dart';
import '../core/theme.dart';
import 'widgets/app_card.dart';
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
      _snack(context.strings.t('nameRequired'));
      return;
    }
    if (value.length > 16) {
      _snack(context.strings.t('nameTooLong'));
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

  void _snack(String message) {
    final messenger = ScaffoldMessenger.of(context);
    messenger.hideCurrentSnackBar();
    messenger.showSnackBar(SnackBar(content: Text(message)));
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
                          PixelAvatar(
                            name: _nameController.text.isEmpty
                                ? 'Steve'
                                : _nameController.text,
                            size: 74,
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
                      Align(
                        alignment: Alignment.centerRight,
                        child: FilledButton.tonalIcon(
                          onPressed: _saveName,
                          icon: const Icon(Icons.check_rounded, size: 19),
                          label: Text(strings.t('save')),
                        ),
                      ),
                    ],
                  ),
                ),
                SectionHeader(
                  title: strings.t('quickServer'),
                  subtitle: strings.t('quickServerHint'),
                ),
                AppCard(
                  padding: const EdgeInsets.all(20),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      TextField(
                        controller: _serverController,
                        keyboardType: TextInputType.url,
                        autocorrect: false,
                        enableSuggestions: false,
                        textInputAction: TextInputAction.done,
                        onSubmitted: (_) => _saveServer(),
                        decoration: InputDecoration(
                          hintText: 'play.example.net',
                          prefixIcon: const Icon(Icons.dns_outlined),
                          helperText: strings.t('serverAddress'),
                        ),
                      ),
                      const SizedBox(height: 14),
                      Align(
                        alignment: Alignment.centerRight,
                        child: FilledButton.tonalIcon(
                          onPressed: _saveServer,
                          icon: const Icon(Icons.check_rounded, size: 19),
                          label: Text(strings.t('save')),
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
                Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    color: scheme.surfaceContainerLow.withOpacity(0.7),
                    borderRadius: BorderRadius.circular(HorizonRadius.card),
                    border: Border.all(
                      color: scheme.outlineVariant.withOpacity(0.45),
                    ),
                  ),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Icon(
                        Icons.info_outline_rounded,
                        size: 18,
                        color: scheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Text(
                          strings.t('legal'),
                          style: textTheme.labelSmall?.copyWith(
                            color: scheme.onSurfaceVariant,
                            height: 1.5,
                            letterSpacing: 0.1,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
