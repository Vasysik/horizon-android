import 'dart:ffi';

import 'package:ffi/ffi.dart';

typedef _StartNative = Int32 Function(Pointer<Utf8> home, Pointer<Utf8> cache);
typedef _StartDart = int Function(Pointer<Utf8> home, Pointer<Utf8> cache);
typedef _ReadyNative = Int32 Function(Pointer<Utf8> home);
typedef _ReadyDart = int Function(Pointer<Utf8> home);

class NativeInstaller {
  NativeInstaller._(this._start, this._ready);

  final _StartDart _start;
  final _ReadyDart _ready;

  static NativeInstaller? _instance;

  static NativeInstaller get instance {
    final existing = _instance;
    if (existing != null) {
      return existing;
    }
    final library = DynamicLibrary.open('libhorizon_android.so');
    final installer = NativeInstaller._(
      library.lookupFunction<_StartNative, _StartDart>('horizon_install_start'),
      library.lookupFunction<_ReadyNative, _ReadyDart>('horizon_install_ready'),
    );
    _instance = installer;
    return installer;
  }

  int start(String home, String cache) {
    final homePointer = home.toNativeUtf8();
    final cachePointer = cache.toNativeUtf8();
    try {
      return _start(homePointer, cachePointer);
    } finally {
      malloc.free(homePointer);
      malloc.free(cachePointer);
    }
  }

  bool isReady(String home) {
    final pointer = home.toNativeUtf8();
    try {
      return _ready(pointer) != 0;
    } finally {
      malloc.free(pointer);
    }
  }
}
