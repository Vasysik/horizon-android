import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'app_settings.dart';
import 'bridge.dart';
import 'install_status.dart';
import 'native_installer.dart';

class AppState extends ChangeNotifier {
  static const String gameVersion = '26.3';

  Bridge? _bridge;
  AppSettings settings = AppSettings();
  InstallStatus? status;
  final ValueNotifier<int> appearanceRevision = ValueNotifier<int>(0);
  bool initializing = true;
  bool installed = false;
  bool startingGame = false;
  String? fatalError;
  String? errorMessage;
  double? speed;
  Duration? eta;
  DateTime lastProgressAt = DateTime.now();

  Timer? _poll;
  bool _serviceStarted = false;
  bool _disposed = false;
  int _lastDone = 0;
  int _lastStage = -1;
  DateTime _lastTick = DateTime.now();

  Bridge get bridge => _bridge!;

  bool get hasBridge => _bridge != null;

  bool get downloading => status?.running ?? false;

  bool get downloadFailed => status?.failed ?? false;

  bool get stalled {
    if (!downloading) {
      return false;
    }
    return DateTime.now().difference(lastProgressAt) >
        const Duration(seconds: 50);
  }

  String get statusPath => '${bridge.filesDir}/horizon-install-status.json';

  Future<void> initialize() async {
    try {
      _bridge = await Bridge.create();
      settings = await AppSettings.load(bridge.filesDir);
      installed = await markersReady();
      status = await InstallStatus.read(statusPath);
      if (status?.running ?? false) {
        _lastDone = status!.done;
        _lastStage = status!.stageId;
        _lastTick = DateTime.now();
        lastProgressAt = DateTime.now();
        await _ensureServiceQuiet();
        _startPolling();
      }
    } catch (error) {
      fatalError = error.toString();
    }
    initializing = false;
    _notify();
  }

  Future<void> refresh() async {
    if (_bridge == null) {
      return;
    }
    installed = await markersReady();
    final next = await InstallStatus.read(statusPath);
    if (next != null) {
      status = next;
      if (next.running && _poll == null) {
        lastProgressAt = DateTime.now();
        _startPolling();
        await _ensureServiceQuiet();
      }
    }
    _notify();
  }

  Future<bool> markersReady() async {
    if (_bridge == null) {
      return false;
    }
    final root = '${bridge.filesDir}/.local/horizon-client/$gameVersion';
    final markers = [
      'horizon-resources.json',
      'android-downloads-v2.json',
      'audio-complete.json',
    ];
    for (final marker in markers) {
      if (!await File('$root/$marker').exists()) {
        return false;
      }
    }
    return true;
  }

  Future<void> startDownload({bool force = false}) async {
    if (_bridge == null || (downloading && !force)) {
      return;
    }
    errorMessage = null;
    speed = null;
    eta = null;
    _lastDone = 0;
    _lastStage = -1;
    _lastTick = DateTime.now();
    lastProgressAt = DateTime.now();
    if (!force || !downloading) {
      status = InstallStatus.starting();
    }
    _notify();
    try {
      await bridge.prepareEngineData();
      final code = NativeInstaller.instance.start(
        bridge.filesDir,
        bridge.cacheDir,
      );
      if (code == -2) {
        throw const InstallerException('missing-engine-data');
      }
      if (code < 0 && code != -3) {
        throw const InstallerException('native-start');
      }
      if (settings.backgroundDownloads) {
        await bridge.startDownloadService(statusPath);
        _serviceStarted = true;
      }
      _startPolling();
    } catch (error) {
      status = InstallStatus(
        state: 'failed',
        stageId: 0,
        stage: 'metadata',
        done: 0,
        total: 0,
        unit: 'none',
        error: error.toString(),
      );
      errorMessage = error.toString();
      _notify();
    }
  }

  Future<void> resumeDownload() async {
    if (!downloading) {
      await startDownload();
      return;
    }
    final restart = stalled;
    lastProgressAt = DateTime.now();
    await _ensureServiceQuiet();
    if (restart) {
      await startDownload(force: true);
    }
    _notify();
  }

  Future<void> launchGame() async {
    if (_bridge == null || startingGame) {
      return;
    }
    if (!installed) {
      errorMessage = 'install-not-ready';
      _notify();
      return;
    }
    startingGame = true;
    errorMessage = null;
    _notify();
    try {
      await syncLauncherConfig();
      await bridge.launchGame();
      await HapticFeedback.mediumImpact();
      await Future<void>.delayed(const Duration(milliseconds: 900));
    } catch (error) {
      errorMessage = error.toString();
    }
    startingGame = false;
    _notify();
  }

  Future<void> syncLauncherConfig() async {
    if (_bridge == null) {
      return;
    }
    final name = displayName;
    final config = {
      'profile': 0,
      'name': name,
      'server': settings.server.trim(),
      'renderer': switch (settings.renderer) {
        'compatibility' => 'vulkan-compatibility',
        'opengl' => 'open-gl',
        _ => 'vulkan',
      },
      'orientation': settings.orientation,
      'input_host': switch (settings.input) {
        'touch' => 'touch',
        'keyboard' => 'keyboard-mouse',
        _ => 'auto',
      },
      'record_performance': false,
    };
    final file = File('${bridge.filesDir}/android-launcher.json');
    final partial = File('${file.path}.partial');
    await partial.writeAsString(jsonEncode(config), flush: true);
    await partial.rename(file.path);
  }

  String get displayName {
    final trimmed = settings.name.trim();
    return trimmed.isEmpty ? 'Steve' : trimmed;
  }

  Future<void> setLocale(String value) async {
    settings.locale = value;
    await _save();
  }

  Future<void> setTheme(String value) async {
    settings.theme = value;
    await _save();
  }

  Future<void> setAccent(String value) async {
    settings.accent = value;
    await _save();
  }

  Future<void> setRenderer(String value) async {
    settings.renderer = value;
    await _save();
  }

  Future<void> setOrientation(String value) async {
    settings.orientation = value;
    await _save();
  }

  Future<void> setInput(String value) async {
    settings.input = value;
    await _save();
  }

  Future<void> setName(String value) async {
    settings.name = value.trim();
    await _save();
  }

  Future<void> setServer(String value) async {
    settings.server = value.trim();
    await _save();
  }

  Future<void> setBackgroundDownloads(bool value) async {
    settings.backgroundDownloads = value;
    await _save();
    if (value) {
      await _ensureServiceQuiet();
    } else if (_serviceStarted) {
      await _stopServiceQuiet();
    }
  }

  Future<void> requestNotifications() async {
    try {
      await bridge.requestNotificationPermission();
    } catch (_) {}
  }

  Future<void> openUrl(String url) async {
    try {
      await bridge.openUrl(url);
    } catch (error) {
      errorMessage = 'link-error';
      _notify();
    }
  }

  void clearError() {
    errorMessage = null;
    _notify();
  }

  Future<void> _save() async {
    if (_bridge == null) {
      return;
    }
    await settings.save(bridge.filesDir);
    appearanceRevision.value++;
    _notify();
  }

  void _startPolling() {
    _poll?.cancel();
    _poll = Timer.periodic(const Duration(milliseconds: 500), (_) {
      _pollOnce();
    });
  }

  Future<void> _pollOnce() async {
    if (_bridge == null) {
      return;
    }
    final next = await InstallStatus.read(statusPath);
    if (next == null) {
      return;
    }
    final changed =
        status == null ||
        status!.state != next.state ||
        status!.stageId != next.stageId ||
        status!.done != next.done ||
        status!.error != next.error;
    final now = DateTime.now();
    if (next.stageId == 1 && _lastStage == 1 && next.done > _lastDone) {
      final seconds = now.difference(_lastTick).inMilliseconds / 1000.0;
      if (seconds > 0) {
        final instant = (next.done - _lastDone) / seconds;
        speed = speed == null ? instant : speed! * 0.72 + instant * 0.28;
        if (speed! > 0 && next.total > 0) {
          final remaining = next.total - next.done;
          eta = Duration(seconds: (remaining / speed!).round());
        }
      }
    }
    if (next.stageId != 1) {
      speed = null;
      eta = null;
    }
    if (next.stageId != _lastStage || next.done != _lastDone) {
      lastProgressAt = now;
    }
    _lastDone = next.done;
    _lastStage = next.stageId;
    _lastTick = now;
    status = next;
    if (next.completed) {
      installed = await markersReady();
      _poll?.cancel();
      _poll = null;
      await _stopServiceQuiet();
    } else if (next.failed) {
      _poll?.cancel();
      _poll = null;
      await _stopServiceQuiet();
    }
    if (changed) {
      _notify();
    }
  }

  Future<void> _ensureServiceQuiet() async {
    if (!settings.backgroundDownloads ||
        !(status?.running ?? false) ||
        _bridge == null) {
      return;
    }
    try {
      await bridge.startDownloadService(statusPath);
      _serviceStarted = true;
    } catch (_) {}
  }

  Future<void> _stopServiceQuiet() async {
    if (!_serviceStarted || _bridge == null) {
      return;
    }
    try {
      await bridge.stopDownloadService();
    } catch (_) {}
    _serviceStarted = false;
  }

  void _notify() {
    if (!_disposed) {
      notifyListeners();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _poll?.cancel();
    appearanceRevision.dispose();
    super.dispose();
  }
}

class InstallerException implements Exception {
  const InstallerException(this.code);

  final String code;

  @override
  String toString() => code;
}
