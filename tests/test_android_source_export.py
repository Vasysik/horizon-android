"""Publication boundaries: private player/build material must never enter exports."""
from pathlib import Path
import sys
import tempfile
import unittest
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import export_android_source as export
from check_public_tree import prohibited


class SourceExportTests(unittest.TestCase):
    def source(self, root):
        for name in ['LICENSE', 'THIRD_PARTY_NOTICES.md', 'Cargo.toml', 'README.md']:
            (root / name).write_text('Original source\n', encoding='utf-8')

    def test_private_inputs_are_excluded_and_workflows_inactive(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.source(root)
            for name in ['.local/game.jar', 'target/game.so', 'packaging/android/local.properties',
                         'packaging/android/debug.keystore', 'tools/.env', 'tools/__pycache__/x.pyc',
                         '.github/workflows/release.yml', 'crates/original.rs']:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('test', encoding='utf-8')
            output = root / '.local/source.zip'
            report = export.export(root, output)
            with zipfile.ZipFile(output) as archive:
                names = set(archive.namelist())
            self.assertIn('crates/original.rs', names)
            self.assertIn('.github/upstream-workflows/release.yml.disabled', names)
            self.assertFalse(any(prohibited(name) for name in names))
            self.assertFalse(any(name.startswith(('.local/', 'target/', '.github/workflows/')) for name in names))
            self.assertEqual(report['upstream_workflows_disabled'], 1)

    def test_credentials_stop_export_without_echoing_secret(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.source(root)
            (root / 'tools').mkdir()
            token = 'ghp_' + 'a' * 36
            (root / 'tools/config.py').write_text(token, encoding='utf-8')
            with self.assertRaises(ValueError) as error:
                export.export(root, root / '.local/source.zip')
            self.assertNotIn(token, str(error.exception))
            self.assertFalse((root / '.local/source.zip').exists())

    def test_public_tree_guard_rejects_android_private_paths(self):
        for name in ['app.apk', 'signing/release.jks', 'control.json', '.env.production', 'a/.gradle/state',
                     'packaging/android/java/net/minecraft/Client.java',
                     'packaging/android/java/dev/horizon/android/OtherActivity.java']:
            self.assertTrue(prohibited(name), name)
        for name in ['LICENSE', 'crates/horizon-client-render/src/touch.rs', '.env.example',
                     'packaging/android/java/dev/horizon/android/HorizonActivity.java',
                     'packaging/android/java/dev/horizon/android/HorizonDocumentsProvider.java']:
            self.assertFalse(prohibited(name), name)


if __name__ == '__main__':
    unittest.main()
