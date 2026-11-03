"""Release scheduling tests never invoke Cargo, Gradle, signing or device tools."""
import importlib.util
from pathlib import Path
import threading
import unittest

spec = importlib.util.spec_from_file_location('android_release',
    Path(__file__).resolve().parents[1] / 'tools/android_release.py')
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseScheduleTests(unittest.TestCase):
    def test_parallel_rust_completes_before_serial_packaging(self):
        barrier = threading.Barrier(2)
        lock = threading.Lock()
        completed = set()
        packaged = []
        owner = threading.get_ident()
        def invoke(abi, rust_only):
            if rust_only:
                barrier.wait(timeout=3)
                with lock:
                    completed.add(abi)
            else:
                self.assertEqual(completed, {'a', 'b'})
                self.assertEqual(threading.get_ident(), owner)
                packaged.append(abi)
        release.build_targets(['a', 'b'], 2, invoke)
        self.assertEqual(packaged, ['a', 'b'])

    def test_sequential_build_does_not_duplicate_compilation(self):
        calls = []
        release.build_targets(['a', 'b'], 1, lambda *call: calls.append(call))
        self.assertEqual(calls, [('a', False), ('b', False)])

    def test_rust_failure_prevents_all_packaging(self):
        calls = []
        def invoke(abi, rust_only):
            calls.append(rust_only)
            raise RuntimeError('compiler failed')
        with self.assertRaises(RuntimeError):
            release.build_targets(['a', 'b'], 2, invoke)
        self.assertTrue(all(calls))
