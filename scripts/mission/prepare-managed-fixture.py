"""Prepare an explicitly modified synthetic asset catalog; preserve release package."""
import argparse
import hashlib
import json
import pathlib
import shutil

parser=argparse.ArgumentParser()
parser.add_argument("--evidence-root", required=True)
ROOT = pathlib.Path(parser.parse_args().evidence_root).resolve()
source = ROOT / "package"
target = ROOT / "payload/package-fixture"
assert not target.exists()
shutil.copytree(source, target)
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
runtime = ROOT / "payload/synthetic-assets/runtime/llama-server.exe"
model = ROOT / "payload/synthetic-assets/gemma-4-12b-it-qat-q4_0.gguf"
catalog_path = target / "Gemma-Assets.json"
catalog = json.loads(catalog_path.read_text(encoding="utf-8-sig"))
catalog["runtimeSha256"] = sha(runtime)
catalog["runtimeBundleSha256"] = hashlib.sha256(("llama-server.exe\0" + sha(runtime) + "\n").encode()).hexdigest()
catalog["model"]["sha256"] = sha(model)
catalog["model"]["size"] = model.stat().st_size
catalog_path.write_text(json.dumps(catalog, indent=2), encoding="utf-8")
manifest_path = target / "MANIFEST.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8-sig"))
for entry in manifest["files"]:
    if entry["path"] == "Gemma-Assets.json":
        entry.update(sha256=sha(catalog_path), bytes=catalog_path.stat().st_size)
manifest["classification"] = "SYNTHETIC_ASSET_CATALOG_FIXTURE_NOT_PERSONAL_DISTRIBUTION"
manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
differences = [p.relative_to(source).as_posix() for p in source.rglob("*") if p.is_file() and sha(p) != sha(target / p.relative_to(source))]
assert sorted(differences) == ["Gemma-Assets.json", "MANIFEST.json"], differences
record = {"sourceCommit": manifest["sourceCommit"], "classification": manifest["classification"], "modifiedFiles": differences,
          "originalManifestSha256": sha(source / "MANIFEST.json"), "fixtureManifestSha256": sha(manifest_path),
          "fixtureSourceSha256": sha(ROOT / "payload/managed-runtime.rs"), "fixtureRuntimeSha256": sha(runtime),
          "unchanged": {name: sha(target / name) for name in ["codex-pencil.exe", "grammar-gemma-broker.exe", "Start-Grammar.ps1", "Install-Grammar.ps1"]}}
(ROOT / "FIXTURE_OVERRIDES.json").write_text(json.dumps(record, indent=2), encoding="utf-8")
