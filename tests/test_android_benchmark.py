"""Behavior of measurement selection; loading and menu frames must not enter results."""
import unittest

from tools.android_benchmark import build_plan, measurement_samples


class MeasurementSamplesTests(unittest.TestCase):
    def test_uses_measurement_boundaries_and_excludes_menus(self):
        def sample(t, world=True, paused=False):
            return {"t_ms": t, "context": {"in_world": world, "paused": paused},
                    "frames": {"avg_fps": 40}}

        rows = [sample(500), sample(1000), sample(1500), sample(2000, paused=True),
                sample(2500, world=False), sample(3000), sample(3500)]
        events = [{"name": "measure-start", "report_t_ms": 1000},
                  {"name": "measure-end", "report_t_ms": 3000}]
        selected, aligned = measurement_samples(rows, events)
        self.assertTrue(aligned)
        self.assertEqual([r["t_ms"] for r in selected], [1500, 3000])

    def test_missing_or_reversed_clock_cannot_be_claimed_aligned(self):
        for events in ([], [{"name": "measure-start", "report_t_ms": 3000},
                            {"name": "measure-end", "report_t_ms": 1000}]):
            with self.subTest(events=events):
                _, aligned = measurement_samples([], events)
                self.assertFalse(aligned)


class BenchmarkPlanTests(unittest.TestCase):
    def test_repeated_cases_keep_unique_trials_and_reverse_build_order(self):
        jobs = build_plan([("lod-off", 16, 0), ("lod-off", 24, 0)], ("old", "new"), 2)
        self.assertEqual(len({job["key"] for job in jobs}), 8)
        self.assertEqual([j["version"] for j in jobs],
                         ["old", "new", "new", "old", "new", "old", "old", "new"])
        self.assertEqual([j["distance"] for j in jobs], [16, 16, 24, 24, 16, 16, 24, 24])


if __name__ == "__main__":
    unittest.main()
