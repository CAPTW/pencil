#!/usr/bin/env python3
"""Behavioral package tests. Mutations stay in an external TemporaryDirectory."""
import argparse
import copy
import re
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import sys
import tempfile
import warnings
import zipfile
from unittest import mock

sys.dont_write_bytecode = True


def run(repo):
    spec = importlib.util.spec_from_file_location("grammar_project_source_under_test", repo / "scripts/project_source.py")
    ps = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ps)
    outcomes = []

    def record(name, body):
        try:
            detail = body()
            outcomes.append({"test": name, "status": "PASS", "detail": detail})
        except Exception as exc:
            outcomes.append({"test": name, "status": "FAIL", "detail": type(exc).__name__ + ": " + str(exc)})

    def rejected(fn, tokens):
        try:
            fn()
        except (ValueError, KeyError, OSError, zipfile.BadZipFile) as exc:
            if not any(token in str(exc) for token in tokens):
                raise AssertionError("wrong failure reason: " + str(exc)) from exc
            return str(exc)
        raise AssertionError("invalid package/operation unexpectedly accepted")

    def check(condition, message):
        if not condition:
            raise AssertionError(message)
        return message

    def inventory(folder):
        return {p.relative_to(folder).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                for p in folder.rglob("*") if p.is_file()}

    state = ps.read_json(repo / "control/state.json")
    inputs = ps.source_inputs(repo)
    for key, bad, tokens in [
        ("p3a_status", "COMPLETE", ["STATE_SEMANTIC_DRIFT:p3a_status"]),
        ("active_phase", "PHASE_9", ["STATE_SEMANTIC_DRIFT:active_phase"]),
        ("active_step", "RUNNING", ["STATE_SEMANTIC_DRIFT:active_step"]),
        ("active_task", "UNAUTHORIZED_TASK", ["STATE_SEMANTIC_DRIFT:active_task"]),
        ("exact_next_task", "GRAMMAR-P3B-P1-WRONG", ["UNSAFE_NEXT_TASK", "STATE_SEMANTIC_DRIFT:exact_next_task"]),
        ("next_task_status", "STARTED", ["STATE_SEMANTIC_DRIFT:next_task_status"]),
        ("next_gate", "P3B_IMPLEMENTATION", ["STATE_SEMANTIC_DRIFT:next_gate"]),
        ("d020_verdict", "D020_AUDIT_FALSE_POSITIVE", ["STATE_SEMANTIC_DRIFT:d020_verdict"]),
        ("automatic_continuation", True, ["NEXT_GATE_FORBIDDEN"]),
        ("next_gate_executed", True, ["NEXT_GATE_FORBIDDEN"]),
    ]:
        changed = copy.deepcopy(state); changed[key] = bad
        record("state_" + key + "_drift_refused", lambda changed=changed,tokens=tokens: rejected(lambda: ps.state_check(changed), tokens))
    for filename, key in [("PRODUCT_CHARTER.md", "CURRENT_PRODUCT_TARGET"),
                          ("FIRST_ADAPTER_DECISION.md", "FIRST_ADAPTER_DECISION"),
                          ("ROADMAP.md", "EXACT_NEXT_TASK")]:
        changed = dict(inputs)
        changed[filename] = re.sub(r"(?m)^" + key + r"=[^\r\n]+$", key + "=CONTRADICTORY_AUTHORITY", changed[filename])
        record("doc_" + key + "_contradiction_refused", lambda changed=changed: rejected(lambda: ps.semantic_check(state, changed), ["DOC_SEMANTIC_DRIFT"]))
        absent = dict(inputs)
        absent[filename] = re.sub(r"(?m)^" + key + r"=[^\r\n]+$", "", absent[filename])
        record("doc_" + key + "_missing_refused", lambda absent=absent: rejected(lambda: ps.semantic_check(state, absent), ["DOC_AUTHORITY_MARKER_MISSING"]))

    with tempfile.TemporaryDirectory(prefix="grammar-source-negative-") as tmp:
        temp = Path(tmp).resolve()
        a, b = temp / "a", temp / "b"
        # A failed baseline aborts safely; downstream rejections must not mask a broken generator.
        baseline = ps.make(repo, a, preview=True)
        ps.make(repo, b, preview=True)
        def fresh_chat_expected_matches_accessible_snapshot():
            def fenced(path):
                return json.loads(path.read_text(encoding="utf-8").split("```json\n", 1)[1].split("```", 1)[0])
            return check(fenced(a / "FRESH_CHAT_SEMANTIC_VERIFICATION_PROMPT.md") == fenced(a / "payload" / ps.NAMES[2]),
                         "fresh chat expectations require only the accessible Source02 snapshot")
        record("fresh_chat_expected_equals_source02", fresh_chat_expected_matches_accessible_snapshot)
        record("deterministic_all_bytes_including_zip", lambda: check(inventory(a) == inventory(b), "two independent outputs identical"))
        record("exact_12_and_fresh_extraction", lambda: check(baseline["logical_sources"] == 12 and baseline["fresh_extraction"] == "PASS", "12 Sources and extraction verified"))
        record("preview_refused_without_opt_in", lambda: rejected(lambda: ps.verify(a, repo, False, True), ["PREVIEW_REFUSED"]))
        record("existing_output_refused", lambda: rejected(lambda: ps.make(repo, a, True), ["OUTPUT_ALREADY_EXISTS"]))
        ident = ps.identity(repo)
        with mock.patch.object(ps, "identity", return_value={**ident, "branch": "candidate/test-only"}):
            record("qualified_nonmain_refused", lambda: rejected(lambda: ps.make(repo, temp / "qualified", False), ["CANONICAL_MAIN_REQUIRED"]))

        def mutated(name, mutate, tokens, extraction=True):
            folder = temp / name
            shutil.copytree(a, folder)
            mutate(folder)
            return rejected(lambda: ps.verify(folder, repo, True, extraction), tokens)

        record("extra_payload_refused", lambda: mutated("extra", lambda x: (x / "payload/99_EXTRA.md").write_text("extra"), ["PAYLOAD_EXACT_SET"]))
        record("missing_payload_refused", lambda: mutated("missing", lambda x: (x / "payload" / ps.NAMES[0]).unlink(), ["PAYLOAD_EXACT_SET"]))
        record("payload_tamper_refused", lambda: mutated("tamper", lambda x: (x / "payload" / ps.NAMES[1]).write_bytes(b"tamper"), ["PAYLOAD_HASH"]))

        def edit_manifest(folder, edit):
            file = folder / "PROJECT_SOURCE_MANIFEST.json"
            data = json.loads(file.read_text(encoding="utf-8")); edit(data)
            file.write_bytes(ps.encode(data))

        record("stale_semantic_inputs_refused", lambda: mutated("semantic", lambda x: edit_manifest(x, lambda m: m.update(semantic_input_digest="0"*64)), ["INPUT_SEMANTICS_STALE"]))

        def stale_identity(folder):
            manifest_path = folder / "PROJECT_SOURCE_MANIFEST.json"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["resolved_state"]["head"] = "0"*40
            snapshot_path = folder / "payload" / ps.NAMES[2]
            old = snapshot_path.read_text(encoding="utf-8")
            prefix, tail = old.split("```json\n", 1)
            _, suffix = tail.split("```", 1)
            snapshot = {k:v for k,v in manifest["resolved_state"].items() if k != "source_content_digest"}
            snapshot_path.write_text(prefix + "```json\n" + ps.encode(snapshot).decode("utf-8") + "```" + suffix, encoding="utf-8", newline="\n")
            payload = {n:(folder / "payload" / n).read_bytes() for n in ps.NAMES}
            manifest["payload"] = {n:{"sha256":ps.sha(data),"bytes":len(data)} for n,data in payload.items()}
            manifest["resolved_state"]["source_content_digest"] = ps.digest_files(payload)
            manifest_path.write_bytes(ps.encode(manifest))
        record("stale_current_git_identity_refused", lambda: mutated("identity", stale_identity, ["GIT_IDENTITY_STALE", "PACKAGE_IDENTITY"]))

        def drop_checksum(folder):
            file = folder / "SHA256SUMS.txt"
            lines = file.read_text(encoding="utf-8").splitlines()
            file.write_text("\n".join(lines[1:]) + "\n", encoding="utf-8", newline="\n")
        record("checksum_coverage_refused", lambda: mutated("coverage", drop_checksum, ["CHECKSUM_COVERAGE"]))

        def duplicate_checksum(folder):
            file = folder / "SHA256SUMS.txt"
            text = file.read_text(encoding="utf-8")
            file.write_text(text + text.splitlines()[0] + "\n", encoding="utf-8", newline="\n")
        record("checksum_duplicate_refused", lambda: mutated("duplicate-sum", duplicate_checksum, ["CHECKSUM_PATH_INVALID"]))

        def bad_zip(folder, kind):
            archive_path = folder / "transport.zip"
            with zipfile.ZipFile(archive_path) as archive:
                entries = [(item, archive.read(item)) for item in archive.namelist()]
            if kind == "duplicate": entries.append(entries[0])
            elif kind == "traversal": entries.append(("../outside.txt", b"malicious"))
            elif kind == "absolute": entries.append(("/outside.txt", b"malicious"))
            elif kind == "content": entries[0] = (entries[0][0], b"modified transport content")
            with warnings.catch_warnings():
                warnings.simplefilter("ignore", UserWarning)
                with zipfile.ZipFile(archive_path, "w", zipfile.ZIP_DEFLATED) as archive:
                    for name, data in entries: archive.writestr(name, data)
        for kind, tokens in [("duplicate", ["ZIP_DUPLICATE"]), ("traversal", ["ZIP_INVENTORY", "ZIP_TRAVERSAL"]),
                             ("absolute", ["ZIP_INVENTORY", "ZIP_TRAVERSAL"]), ("content", ["ZIP_BYTES"])]:
            record("zip_" + kind + "_refused", lambda kind=kind,tokens=tokens: mutated("zip-" + kind, lambda x: bad_zip(x, kind), tokens))
        record("traversal_wrote_nothing", lambda: check(not (temp / "outside.txt").exists(), "no escape output"))
    return outcomes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--evidence-dir", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()
    try:
        outcomes = run(repo)
        result = {"status": "PASS" if all(x["status"] == "PASS" for x in outcomes) else "FAIL",
                  "scope": "candidate preview package tests; not product or UI acceptance", "tests": outcomes}
    except Exception as exc:
        result = {"status":"FAIL", "error":type(exc).__name__ + ": " + str(exc)}
    text = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    if args.evidence_dir:
        evidence = args.evidence_dir.resolve()
        if evidence.is_relative_to(repo):
            raise ValueError("evidence must remain outside repository")
        evidence.mkdir(parents=True, exist_ok=True)
        (evidence / "project_source_behavior_tests.json").write_text(text, encoding="utf-8", newline="\n")
    print(text, end="")
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
