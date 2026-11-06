import 'dart:convert';
import 'dart:io';

class AppSettings {
  AppSettings({
    this.locale = 'system',
    this.theme = 'system',
    this.accent = 'grass',
    this.renderer = 'vulkan',
    this.orientation = 'landscape',
    this.input = 'auto',
    this.backgroundDownloads = true,
    this.name = 'Steve',
    this.server = '',
    this.skinPath = '',
    this.skinSlim = false,
    this.controlsInitialized = false,
  });

  String locale;
  String theme;
  String accent;
  String renderer;
  String orientation;
  String input;
  bool backgroundDownloads;
  String name;
  String server;
  String skinPath;
  bool skinSlim;
  bool controlsInitialized;

  static const fileName = 'horizon-ui-settings.json';

  factory AppSettings.fromJson(Map<String, dynamic> json) {
    return AppSettings(
      locale: _string(json['locale'], 'system'),
      theme: _string(json['theme'], 'system'),
      accent: _string(json['accent'], 'grass'),
      renderer: _string(json['renderer'], 'vulkan'),
      orientation: _string(json['orientation'], 'landscape'),
      input: _string(json['input'], 'auto'),
      backgroundDownloads: json['backgroundDownloads'] is bool
          ? json['backgroundDownloads'] as bool
          : true,
      name: _string(json['name'], 'Steve'),
      server: _string(json['server'], ''),
      skinPath: _string(json['skinPath'], ''),
      skinSlim: json['skinSlim'] is bool ? json['skinSlim'] as bool : false,
      controlsInitialized: json['controlsInitialized'] is bool
          ? json['controlsInitialized'] as bool
          : false,
    );
  }

  static String _string(Object? value, String fallback) {
    return value is String && value.isNotEmpty ? value : fallback;
  }

  Map<String, dynamic> toJson() {
    return {
      'locale': locale,
      'theme': theme,
      'accent': accent,
      'renderer': renderer,
      'orientation': orientation,
      'input': input,
      'backgroundDownloads': backgroundDownloads,
      'name': name,
      'server': server,
      'skinPath': skinPath,
      'skinSlim': skinSlim,
      'controlsInitialized': controlsInitialized,
    };
  }

  static Future<AppSettings> load(String directory) async {
    try {
      final file = File('$directory/$fileName');
      if (await file.exists()) {
        final decoded = jsonDecode(await file.readAsString());
        if (decoded is Map) {
          return AppSettings.fromJson(Map<String, dynamic>.from(decoded));
        }
      }
    } catch (_) {}
    return AppSettings();
  }

  Future<void> save(String directory) async {
    final folder = Directory(directory);
    if (!await folder.exists()) {
      await folder.create(recursive: true);
    }
    final partial = File('$directory/$fileName.partial');
    await partial.writeAsString(jsonEncode(toJson()), flush: true);
    await partial.rename('$directory/$fileName');
  }
}
