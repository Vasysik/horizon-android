"""Machine-consumed lifecycle contract for NativeActivity input hotplug."""
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
ANDROID = '{http://schemas.android.com/apk/res/android}'


class ActivityContract(unittest.TestCase):
    def test_input_hotplug_is_delivered_without_recreating_native_activity(self):
        root = ET.parse(ROOT / 'packaging/android/AndroidManifest.xml').getroot()
        activity = next(a for a in root.findall('application/activity')
                        if a.get(ANDROID + 'name') == 'dev.horizon.android.HorizonActivity')
        changes = set(activity.get(ANDROID + 'configChanges', '').split('|'))
        self.assertTrue({'keyboard', 'keyboardHidden', 'navigation'} <= changes)


if __name__ == '__main__':
    unittest.main()
