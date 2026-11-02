"""Cross-platform release smoke using only isolated, generated browser profiles."""
import argparse
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    args = parser.parse_args()
    suffix = ".exe" if os.name == "nt" else ""
    binary = (args.bin_dir / f"omoi{suffix}").resolve()
    generator = (args.bin_dir / "examples" / f"synthetic{suffix}").resolve()
    with tempfile.TemporaryDirectory(prefix="omoi-release-smoke-") as temporary:
        root = Path(temporary)
        env = os.environ.copy()
        for key in ("LOCALAPPDATA", "APPDATA", "HOME", "XDG_CONFIG_HOME"):
            env[key] = str(root / "empty-roots")
        fixtures = [root / name for name in ("first", "second")]
        logical = []
        for fixture in fixtures:
            subprocess.run([str(generator), "--out", str(fixture), "--views", "1000", "--items", "400"], env=env, check=True, capture_output=True)
            c = sqlite3.connect(fixture / "History")
            logical.append(c.execute("SELECT * FROM visits ORDER BY id").fetchall())
            by_item = {}
            for _, item, timestamp, _ in logical[-1]:
                if item in by_item:
                    assert timestamp - by_item[item] > 1800 * 1000000
                by_item[item] = timestamp
            c.close()
            text = (fixture / "home/config.toml").read_text(encoding="utf-8")
            assert str(fixture.resolve()).replace("\\", "\\\\") in text
        assert logical[0] == logical[1], "generator is not deterministic"
        env["OMOI_HOME"] = str(fixtures[0] / "home")

        def run(*arguments):
            result = subprocess.run([str(binary), *arguments], env=env, check=True, capture_output=True)
            return json.loads(result.stdout)

        first = run("collect")
        assert first["items_inserted"] == 400 and first["views_inserted"] == 1000
        assert first["rows_read"] == 1200 and first["rows_rejected"] == 100
        second = run("collect")
        assert second["views_inserted"] == 0 and second["profiles_unchanged"] == 2
        exported = run("export", "--json")
        assert len(exported) == 1000
        shelves = run("recommend")
        assert len(shelves["rediscover"]) > 0
        assert shelves["continue_watching"] == [] and shelves["timebox"] == []
        print("PASS: deterministic fixtures, spacing, counts, rerun, export and local suggestions")


if __name__ == "__main__":
    main()
