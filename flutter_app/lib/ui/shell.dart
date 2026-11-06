import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core/app_state.dart';
import '../core/app_strings.dart';
import 'account_screen.dart';
import 'home_screen.dart';
import 'settings_screen.dart';

class LauncherShell extends StatefulWidget {
  const LauncherShell({super.key, required this.state});

  final AppState state;

  @override
  State<LauncherShell> createState() => _LauncherShellState();
}

class _LauncherShellState extends State<LauncherShell>
    with WidgetsBindingObserver {
  final PageStorageBucket _bucket = PageStorageBucket();
  int _index = 0;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      widget.state.refresh();
    }
  }

  void _select(int index) {
    if (index == _index) {
      return;
    }
    HapticFeedback.selectionClick();
    setState(() => _index = index);
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final strings = context.strings;
    return PopScope(
      canPop: _index == 0,
      onPopInvokedWithResult: (didPop, result) {
        if (!didPop && _index != 0) {
          setState(() => _index = 0);
        }
      },
      child: Scaffold(
        extendBody: true,
        body: PageStorage(
          bucket: _bucket,
          child: AnimatedSwitcher(
            duration: const Duration(milliseconds: 380),
            switchInCurve: Curves.easeOutCubic,
            switchOutCurve: Curves.easeInCubic,
            transitionBuilder: (child, animation) {
              final fade = CurvedAnimation(
                parent: animation,
                curve: const Interval(0, 0.85, curve: Curves.easeOut),
              );
              final slide = Tween<Offset>(
                begin: const Offset(0, 0.018),
                end: Offset.zero,
              ).animate(animation);
              return FadeTransition(
                opacity: fade,
                child: SlideTransition(
                  position: slide,
                  child: ScaleTransition(
                    scale: Tween<double>(
                      begin: 0.992,
                      end: 1,
                    ).animate(animation),
                    child: child,
                  ),
                ),
              );
            },
            child: KeyedSubtree(
              key: ValueKey(_index),
              child: switch (_index) {
                1 => AccountScreen(
                  key: const PageStorageKey('account'),
                  state: widget.state,
                ),
                2 => SettingsScreen(
                  key: const PageStorageKey('settings'),
                  state: widget.state,
                ),
                _ => HomeScreen(
                  key: const PageStorageKey('home'),
                  state: widget.state,
                  onNavigate: _select,
                ),
              },
            ),
          ),
        ),
        bottomNavigationBar: SafeArea(
          minimum: const EdgeInsets.fromLTRB(16, 0, 16, 12),
          child: Container(
            decoration: BoxDecoration(
              color: scheme.surfaceContainer.withOpacity(0.94),
              borderRadius: BorderRadius.circular(28),
              border: Border.all(color: scheme.outlineVariant.withOpacity(0.6)),
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withOpacity(0.18),
                  blurRadius: 28,
                  offset: const Offset(0, 12),
                ),
              ],
            ),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(28),
              child: NavigationBar(
                selectedIndex: _index,
                onDestinationSelected: _select,
                backgroundColor: Colors.transparent,
                elevation: 0,
                destinations: [
                  NavigationDestination(
                    icon: const Icon(Icons.home_outlined),
                    selectedIcon: const Icon(Icons.home_rounded),
                    label: strings.t('navHome'),
                  ),
                  NavigationDestination(
                    icon: const Icon(Icons.person_outline_rounded),
                    selectedIcon: const Icon(Icons.person_rounded),
                    label: strings.t('navAccount'),
                  ),
                  NavigationDestination(
                    icon: const Icon(Icons.tune_rounded),
                    selectedIcon: const Icon(Icons.tune_rounded),
                    label: strings.t('navSettings'),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
