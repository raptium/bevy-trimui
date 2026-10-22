"""Local end-to-end App packaging, install and collection without a device."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
APP = "bevy-trimui-demo"


class DeviceToolsTest(unittest.TestCase):
    def test_package_install_preserves_saves_and_collects_matching_pixels(self):
        with tempfile.TemporaryDirectory(prefix="brick demo ") as temporary:
            base = Path(temporary)
            binary = base / "demo"
            binary.write_bytes(b"test-only executable fixture\n")
            binary.chmod(0o755)
            build = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
            binary.with_suffix(".build.json").write_text(json.dumps(build))
            package = base / "packages" / APP
            self.run_tool("package_bevy_spike.py", "--binary", binary,
                          "--dest", package.parent, "--seconds", "2")
            config = json.loads((package / "config.json").read_text())
            self.assertEqual(config["label"], "Bevy TRIMUI Demo")
            self.assertTrue((package / APP).is_file())
            subprocess.run(["sh", "-n", str(package / "launch.sh")], check=True)

            remote_apps = base / "remote Apps"
            remote_apps.mkdir()
            installed = remote_apps / APP
            commands = base / "commands"
            commands.mkdir()
            # shasum is the macOS equivalent; Linux already has sha256sum.
            if sys.platform == "darwin":
                checksum = commands / "sha256sum"
                checksum.write_text('#!/bin/sh\nexec shasum -a 256 "$@"\n')
                checksum.chmod(0o755)
            wrapper = base / "ssh-fixture"
            wrapper.write_text(
                "#!/usr/bin/env python3\nimport os,subprocess,sys\n"
                "command = sys.argv[-1].replace('/mnt/SDCARD/Apps/bevy-trimui-demo', "
                "__import__('shlex').quote(os.environ['BEVY_TRIMUI_TEST_APP']))\n"
                "command = command.replace('cat /sys/kernel/debug/pvr/status', "
                "\"printf 'GPU fixture OK\\\\n'\")\n"
                "sys.exit(subprocess.run(['sh', '-c', command]).returncode)\n"
            )
            wrapper.chmod(0o755)
            env = dict(os.environ, BEVY_TRIMUI_SSH_WRAPPER=str(wrapper),
                       BEVY_TRIMUI_TEST_APP=str(installed),
                       PATH=str(commands) + os.pathsep + os.environ["PATH"])
            for attempt in range(2):
                self.run_tool("install_app.py", "--package", package,
                              "--remote-dir", remote_apps, env=env)
                if not attempt:
                    (installed / "save/keep.txt").write_text("existing progress")
            self.assertEqual((installed / "save/keep.txt").read_text(), "existing progress")
            self.assertEqual((installed / APP).read_bytes(), binary.read_bytes())

            run = "run-20261022-120000-123"
            run_dir = installed / "save" / run
            run_dir.mkdir()
            (installed / "save/last-run.txt").write_text(run + "\n")
            (run_dir / "build.json").write_text(json.dumps(build))
            (run_dir / "exit-code.txt").write_text("0\n")
            (run_dir / "spike.log").write_text("stage=display_snapshot_saved\n")
            pixels = bytearray(bytes((13, 13, 25)) * (1024 * 768))
            for x, y, size, color in ((384, 256, 256, (255, 127, 0)),
                                      (480, 152, 64, (26, 102, 255))):
                for row in range(y, y + size):
                    offset = (row * 1024 + x) * 3
                    pixels[offset:offset + size * 3] = bytes(color) * size
            (run_dir / "display.ppm").write_bytes(b"P6\n1024 768\n255\n" + pixels)
            result = self.run_tool("collect_bevy_spike.py", "--output", base / "collected", env=env)
            self.assertIn("BEVY_SPIKE_PPM_PASSED", result.stdout)
            collected = json.loads((base / "collected/collection.json").read_text())
            self.assertEqual(collected["binary_sha256"], build["binary_sha256"])
            self.assertEqual(collected["exit_code"], 0)

            # Consumers have their own executable identity and collection logic.
            custom = "custom-app-runner"
            (package / APP).rename(package / custom)
            self.run_tool("install_app.py", "--package", package,
                          "--remote-dir", remote_apps, "--executable", custom, env=env)
            self.assertEqual((installed / custom).read_bytes(), binary.read_bytes())
            self.assertFalse((installed / APP).exists())
            self.assertEqual((installed / "save/keep.txt").read_text(), "existing progress")

    def run_tool(self, name, *arguments, env=None):
        result = subprocess.run([sys.executable, str(ROOT / "tools/device" / name),
                                 *map(str, arguments)], cwd=ROOT, env=env,
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return result


if __name__ == "__main__":
    unittest.main()
