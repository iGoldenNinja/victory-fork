import runpy
import unittest
from pathlib import Path


timeline = runpy.run_path(str(Path(__file__).with_name("descriptor-trace-order.py")))["timeline"]


class DescriptorOrderTests(unittest.TestCase):
    def test_preview_uses_preceding_submission_and_current_descriptor(self):
        entries = [
            "binding id=2 frame=12 role=writer shader=a native=55",
            "marker id=3 frame=12 role=target-preview native=55",
            "binding id=1 frame=11 role=writer shader=a native=55",
            "binding id=5 frame=11 role=reader shader=b native=88",
            "binding id=4 frame=11 role=reader shader=c native=55",
            "submit id=2 frame=12 fence=20 position=0 native=55",
            "submit id=3 frame=12 fence=15 position=0 native=0",
            "submit id=5 frame=11 fence=14 position=1 native=55",
            "submit id=1 frame=11 fence=14 position=0 native=55",
            "submit id=4 frame=11 fence=14 position=1 native=0",
        ]
        rows = timeline("descriptor trace " + entry for entry in entries)
        seen = [(event, resource, stable, writer)
                for _, _, _, event, _, resource, stable, writer, _ in rows]
        self.assertEqual(seen, [
            (1, 1, True, None),
            (5, 1, False, 1),      # Changed to the writer's resource.
            (4, None, False, None),  # Cleared before submission.
            (3, 1, None, 1),       # Retained preview, before writer 2.
            (2, 1, True, None),
        ])


if __name__ == "__main__":
    unittest.main()
