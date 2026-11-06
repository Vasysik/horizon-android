import 'dart:convert';
import 'dart:io';

class InstallStatus {
  const InstallStatus({
    required this.state,
    required this.stageId,
    required this.stage,
    required this.done,
    required this.total,
    required this.unit,
    required this.error,
  });

  final String state;
  final int stageId;
  final String stage;
  final int done;
  final int total;
  final String unit;
  final String error;

  bool get running => state == 'running';
  bool get completed => state == 'done';
  bool get failed => state == 'failed';
  bool get active => running || completed || failed;

  double? get progress {
    if (total <= 0) {
      return null;
    }
    return (done / total).clamp(0.0, 1.0).toDouble();
  }

  int get percent => ((progress ?? 0) * 100).round();

  String get stageKey {
    switch (stageId) {
      case 0:
        return 'stageMetadata';
      case 1:
        return 'stageDownload';
      case 2:
        return 'stageExtract';
      case 3:
        return 'stageWorldData';
      case 4:
        return 'stageControls';
      case 5:
        return 'stageFinalize';
      default:
        return 'preparing';
    }
  }

  static InstallStatus? fromJson(Map<String, dynamic>? json) {
    if (json == null) {
      return null;
    }
    return InstallStatus(
      state: json['state'] is String ? json['state'] as String : 'idle',
      stageId: json['stage_id'] is num ? (json['stage_id'] as num).toInt() : 0,
      stage: json['stage'] is String ? json['stage'] as String : 'metadata',
      done: json['done'] is num ? (json['done'] as num).toInt() : 0,
      total: json['total'] is num ? (json['total'] as num).toInt() : 0,
      unit: json['unit'] is String ? json['unit'] as String : 'none',
      error: json['error'] is String ? json['error'] as String : '',
    );
  }

  static Future<InstallStatus?> read(String path) async {
    try {
      final file = File(path);
      if (!await file.exists()) {
        return null;
      }
      final decoded = jsonDecode(await file.readAsString());
      if (decoded is Map) {
        return InstallStatus.fromJson(Map<String, dynamic>.from(decoded));
      }
    } catch (_) {}
    return null;
  }

  static InstallStatus starting() {
    return const InstallStatus(
      state: 'running',
      stageId: 0,
      stage: 'metadata',
      done: 0,
      total: 0,
      unit: 'none',
      error: '',
    );
  }
}
