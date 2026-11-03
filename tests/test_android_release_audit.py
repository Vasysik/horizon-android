import importlib.util
import io
from pathlib import Path
import tempfile
import unittest
import zipfile
import sys
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('android_release_audit', Path(__file__).resolve().parents[1] / 'tools/check_android_release.py')
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


def archive(files):
    output = io.BytesIO()
    with zipfile.ZipFile(output, 'w') as z:
        for name, data in files.items():
            z.writestr(name, data)
    return output.getvalue()


class ReleaseAuditTests(unittest.TestCase):
    def test_public_apk_accepts_generated_tables_and_rejects_original_assets(self):
        build_spec = importlib.util.spec_from_file_location('android_build', Path(__file__).resolve().parents[1] / 'tools/android_build.py')
        build = importlib.util.module_from_spec(build_spec)
        build_spec.loader.exec_module(build)
        self.assertTrue(build.generated_engine_file('worldgen/blocks/structure-blocks.json'))
        self.assertFalse(build.generated_engine_file('worldgen/structure/minecraft/village/test.nbt'))
        with tempfile.TemporaryDirectory() as folder, patch.dict(sys.modules, {'android_build': build}):
            path = Path(folder) / 'public.apk'
            files = {'AndroidManifest.xml': b'test', 'assets/engine-data.zip': archive({'facts.json': b'{}'})}
            path.write_bytes(archive(files))
            self.assertEqual(audit.inspect_payload(path, public=True)['findings'], [])
            for name in ['worldgen/structure/minecraft/test.nbt', 'worldgen/biomes.json', 'raw.jar', 'unknown.json']:
                files['assets/engine-data.zip'] = archive({name: b'original content'})
                path.write_bytes(archive(files))
                self.assertIn('original_game_resource', {f['kind'] for f in audit.inspect_payload(path, public=True)['findings']})
            files['assets/bedrock-controls.zip'] = archive({'ui/jump.png': b'texture'})
            path.write_bytes(archive(files))
            with self.assertRaises(ValueError):
                audit.inspect_payload(path, public=True)
    def inspect(self, engine=None, extra=None):
        files = {'AndroidManifest.xml': b'test manifest',
                 'assets/engine-data.zip': archive(engine or {'blocks.json': b'{}'}),
                 'assets/bedrock-controls.zip': archive({'LICENSE.md': b'license'}),
                 **(extra or {})}
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'test.apk'
            path.write_bytes(archive(files))
            return audit.inspect_payload(path)

    def test_resource_only_apk_has_no_private_findings(self):
        self.assertEqual(self.inspect()['findings'], [])

    def test_androidx_build_profiles_are_allowed(self):
        self.assertEqual(self.inspect(extra={'assets/dexopt/baseline.prof': b'compiled profile'})['findings'], [])

    def test_nested_player_data_and_unexpected_asset_are_rejected(self):
        report = self.inspect({'profiles/account.json': b'{}'}, {'assets/settings.json': b'{}'})
        self.assertEqual({f['kind'] for f in report['findings']}, {'private_file', 'unexpected_asset'})

    def test_credential_in_native_library_is_rejected_without_reporting_value(self):
        token = b'ghp_' + b'X' * 36
        report = self.inspect(extra={'lib/arm64-v8a/libtest.so': b'ELF\x00' + token})
        self.assertEqual(report['findings'][0]['kind'], 'github_token')
        self.assertNotIn(token.decode(), str(report))

    def test_personal_build_path_and_private_key_are_rejected(self):
        key = b'-----BEGIN ' + b'PRIVATE KEY-----'
        report = self.inspect(extra={'lib/arm64-v8a/libtest.so': b'C:/Users/example/.cargo/source.rs\x00' + key})
        self.assertEqual({f['kind'] for f in report['findings']}, {'personal_build_path', 'private_key'})

    def test_nested_traversal_is_rejected(self):
        with self.assertRaises(ValueError):
            self.inspect({'../account.json': b'{}'})
