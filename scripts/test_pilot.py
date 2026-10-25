"""Offline tests for the pilot harness's handling of child output. No network, no build.

Run: python3 -m unittest scripts/test_pilot.py"""
import io
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pilot  # noqa: E402


def run(lines):
    sink = io.StringIO()
    return pilot.read_events(lines, sink), sink.getvalue()


class ReadEvents(unittest.TestCase):
    def test_counts_emitted_events_and_keeps_the_summary(self):
        (emitted, summary, error), written = run(
            ['{"kind":"emitted","slot":1}\n', '{"kind":"emitted","slot":2}\n',
             '{"kind":"summary","target_reached":true}\n'])
        self.assertEqual((emitted, error), (2, None))
        self.assertTrue(summary["target_reached"])
        self.assertEqual(written.count("\n"), 3)

    def test_garbage_stops_the_read_without_echoing_it(self):
        secret = "https://user:token@provider.example/path"
        (emitted, summary, error), written = run(
            ['{"kind":"emitted","slot":1}\n', secret + "\n", '{"kind":"emitted","slot":3}\n'])
        self.assertEqual(emitted, 1)
        self.assertIsNone(summary)
        self.assertIn("not a pilot event", error)
        self.assertNotIn("provider.example", written + error)

    def test_json_that_is_not_an_event_object_is_an_error(self):
        for line in ("[1,2,3]\n", "42\n", "null\n", '{"kind":"other"}\n', "\n"):
            with self.subTest(line=line):
                _, _, error = run([line])[0]
                self.assertIsNotNone(error)

    def test_a_run_with_no_summary_reports_none(self):
        (emitted, summary, error), _ = run(['{"kind":"emitted","slot":1}\n'])
        self.assertEqual((emitted, summary, error), (1, None, None))


if __name__ == "__main__":
    unittest.main()
