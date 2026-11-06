import 'package:flutter/services.dart';

class Bridge {
  Bridge._(this.filesDir, this.cacheDir, this.versionName);

  final String filesDir;
  final String cacheDir;
  final String versionName;

  static const MethodChannel _channel = MethodChannel('dev.horizon.ui/bridge');

  static Future<Bridge> create() async {
    final values =
        await _channel.invokeMapMethod<String, dynamic>('paths') ?? const {};
    final files = values['filesDir'];
    final cache = values['cacheDir'];
    if (files is! String || cache is! String) {
      throw StateError('Android paths are unavailable');
    }
    return Bridge._(
      files,
      cache,
      values['versionName'] is String
          ? values['versionName'] as String
          : '1.0.0',
    );
  }

  Future<void> prepareEngineData() {
    return _channel.invokeMethod<void>('prepareEngineData');
  }

  Future<void> startDownloadService(String statusPath) {
    return _channel.invokeMethod<void>('startDownloadService', {
      'statusPath': statusPath,
    });
  }

  Future<void> stopDownloadService() {
    return _channel.invokeMethod<void>('stopDownloadService');
  }

  Future<void> requestNotificationPermission() {
    return _channel.invokeMethod<void>('requestNotificationPermission');
  }

  Future<void> launchGame() {
    return _channel.invokeMethod<void>('launchGame');
  }

  Future<void> openUrl(String url) {
    return _channel.invokeMethod<void>('openUrl', {'url': url});
  }
}
