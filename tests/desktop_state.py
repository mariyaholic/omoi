"""Verify asynchronous WebView2 profile cleanup without starting an application."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "verify_desktop", Path(__file__).resolve().parents[1] / "scripts/verify_desktop.py")
desktop = importlib.util.module_from_spec(spec)
spec.loader.exec_module(desktop)


class ProfileCleanup(unittest.TestCase):
    def test_child_disappearing_does_not_mean_the_profile_is_gone(self):
        profile = Path(tempfile.gettempdir()) / "omoi-synthetic-profile"
        with patch.object(desktop.shutil, "rmtree", side_effect=[FileNotFoundError(), None]) as remove:
            with patch.object(Path, "exists", return_value=True), patch.object(desktop.time, "sleep"):
                desktop.remove_closed_profile(profile)
        self.assertEqual(remove.call_count, 2)

    def test_missing_profile_is_already_clean(self):
        profile = Path(tempfile.gettempdir()) / "omoi-synthetic-profile"
        with patch.object(desktop.shutil, "rmtree", side_effect=FileNotFoundError()) as remove:
            with patch.object(Path, "exists", return_value=False):
                desktop.remove_closed_profile(profile)
        self.assertEqual(remove.call_count, 1)


if __name__ == "__main__":
    unittest.main()
