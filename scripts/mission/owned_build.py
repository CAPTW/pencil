"""Own MSVC build helpers through teardown; never use for product acceptance.

Usage: python scripts/mission/owned_build.py --output <fresh-evidence> --timeout
1200 -- pwsh -NoProfile -File scripts/mission/build-receipt.ps1 ...
No registry/telemetry settings are changed. Only the exact installed MSVC helper
inside our kill-on-close Job may be reaped as declared build teardown. All other
leftovers still fail. A reaped helper is not reported as a natural exit.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess

from owned_trial import run


def installed_helper():
    vswhere = Path(os.environ["ProgramFiles(x86)"]) / "Microsoft Visual Studio/Installer/vswhere.exe"
    result = subprocess.run([str(vswhere), "-latest", "-products", "*", "-requires",
                             "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                             "-property", "installationPath"], capture_output=True,
                            text=True, check=True, timeout=15,
                            creationflags=subprocess.CREATE_NO_WINDOW)
    installation = result.stdout.strip()
    if not installation or len(installation.splitlines()) != 1:
        raise ValueError("MSVC installation unavailable or ambiguous")
    root = Path(installation).resolve(strict=True)
    version = (root / "VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt").read_text().strip()
    if not version or any(c not in "0123456789." for c in version):
        raise ValueError("Unexpected installed MSVC version")
    helper = (root / "VC/Tools/MSVC" / version / "bin/Hostx64/x64/vctip.exe").resolve(strict=True)
    if not helper.is_relative_to(root):
        raise ValueError("MSVC helper outside installation")
    return str(helper)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True)
    parser.add_argument("--timeout", type=float, default=1200)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or args.timeout <= 0:
        parser.error("a build command and positive timeout are required")
    result = run(command, args.output, args.timeout, owned_helper_images=[installed_helper()])
    print(json.dumps(result))
    raise SystemExit(0 if result["status"] in ("PASS", "BUILD_COMPLETE_HELPERS_REAPED")
                     and result["cleanup"] == "COMPLETE" else 1)
