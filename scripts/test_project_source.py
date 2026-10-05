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
    def legacy_original_integrity():
        appendix = inputs["LEGACY_SOURCE_INTEGRATION.md"]
        originals = re.findall(r"### ([^\n]+)\n\nSHA256: `([a-f0-9]{64})`\n\n```text\n([\s\S]*?)\n```", appendix)
        check(len(originals) == 7, "exact seven provided legacy originals")
        check({name for name, _, _ in originals} == set(ps.RETAINED_HISTORY[1:]), "missing multi-provider file must not be invented")
        for name, digest, content in originals:
            check(hashlib.sha256(content.encode("utf-8")).hexdigest() == digest, "legacy original bytes: " + name)
        check("NOT_READ_NOT_INTEGRATED" in inputs["SOURCE_LIFECYCLE.md"], "missing source must remain explicit")
        return "all seven originals hash-match; eighth remains missing"
    record("legacy_original_integrity_and_missing_scope", legacy_original_integrity)
    record("current_state_exact_semantics", lambda: ps.state_check(state))
    f06 = next(d for d in state["known_defects"] if d["id"] == "F-06")
    if "qualification_scope" in f06:
        for name, mutate, token in (
            ("missing_scope", lambda d: d.pop("qualification_scope"), "VERDICT_DRIFT:F-06"),
            ("writer_overclaim", lambda d: d["qualification_scope"].update(writer="ALL_WRITER_DOCUMENTS_QUALIFIED"), "F06_SCOPE_DRIFT"),
            ("stale_summary", lambda d: d.update(verdict="OWNED_QUALIFIED_EDIT_AND_RICHEDIT_TARGET_SIDE_ACCEPT_SYNTHETIC_VERIFIED_OTHER_EDITORS_UNQUALIFIED"), "VERDICT_DRIFT:F-06"),
        ):
            changed = copy.deepcopy(state)
            mutate(next(d for d in changed["known_defects"] if d["id"] == "F-06"))
            record("scoped_f06_" + name + "_refused", lambda changed=changed,token=token: rejected(lambda: ps.state_check(changed), [token]))
        historical = json.loads(ps.git(repo, "show", "167e5f4:control/state.json"))
        record("historical_pre_scope_state_preserved", lambda: ps.state_check(historical))
        unsaved_scope = json.loads(ps.git(repo, "show", "5a238f2:control/state.json"))
        record("historical_unsaved_writer_scope_preserved", lambda: ps.state_check(unsaved_scope))
        saved_scope = json.loads(ps.git(repo, "show", "82b798d:control/state.json"))
        record("historical_saved_plain_writer_scope_preserved", lambda: ps.state_check(saved_scope))
        uniform_scope = json.loads(ps.git(repo, "show", "2d71c83:control/state.json"))
        record("historical_uniform_writer_scope_preserved", lambda: ps.state_check(uniform_scope))
    if state["p3a_status"] in (ps.APPLY_STATUS, ps.GLOBAL_APPLY_STATUS, ps.EDIT_APPLY_STATUS):
        changed = copy.deepcopy(state)
        next(d for d in changed["known_defects"] if d["id"] == "F-06")["verdict"] = "CONTAINED_COPY_ONLY_NATIVE_NOT_QUALIFIED"
        record("owned_apply_mixed_qualification_refused", lambda: rejected(lambda: ps.state_check(changed), ["VERDICT_DRIFT:F-06"]))
        changed_scope = copy.deepcopy(state)
        changed_scope["native_capture_activation"] = "ALL_WINDOWS_EDITORS_QUALIFIED"
        record("owned_apply_general_editor_overclaim_refused", lambda: rejected(lambda: ps.state_check(changed_scope), ["STATE_SEMANTIC_DRIFT:native_capture_activation"]))
    if state["p3a_status"] in (ps.GLOBAL_APPLY_STATUS, ps.EDIT_APPLY_STATUS):
        historical_apply = json.loads(ps.git(repo, "show", "07541cb2aac7ee85a4749decfc7e0d55cd8591b5:control/state.json"))
        record("historical_richedit_qualification_preserved", lambda: ps.state_check(historical_apply))
        for verdict in ("ALL_EDITORS_QUALIFIED", "OWNED_RICHEDIT_TARGET_SIDE_APPLY_SYNTHETIC_VERIFIED_OTHER_EDITORS_UNQUALIFIED"):
            mixed = copy.deepcopy(state)
            next(d for d in mixed["known_defects"] if d["id"] == "F-06")["verdict"] = verdict
            record("global_apply_rejects_" + verdict, lambda mixed=mixed: rejected(lambda: ps.state_check(mixed), ["VERDICT_DRIFT:F-06"]))
    if state["p3a_status"] == ps.EDIT_APPLY_STATUS:
        prior_task = copy.deepcopy(state)
        prior_task["exact_next_task"] = ps.GLOBAL_APPLY_TASK
        record("historical_multiline_task_preserved", lambda: ps.state_check(prior_task))
        browser_task = copy.deepcopy(state)
        browser_task["exact_next_task"] = ps.GLOBAL_BROWSER_TASK
        record("authorized_browser_task_accepted", lambda: ps.state_check(browser_task))
        historical_single = json.loads(ps.git(repo, "show", "adecefdb618640c0dff247228255a74520339ae1:control/state.json"))
        record("historical_singleline_qualification_preserved", lambda: ps.state_check(historical_single))
        unsupported = copy.deepcopy(state)
        unsupported["native_capture_activation"] = "ALL_MULTILINE_EDIT_ENABLED"
        record("multiline_style_overclaim_refused", lambda: rejected(lambda: ps.state_check(unsupported), ["STATE_SEMANTIC_DRIFT:native_capture_activation"]))
    if ps.mission_state(state):
        for key, bad, token in [
            ("mission_id", "UNAUTHORIZED-MISSION", "MISSION_NOT_AUTHORIZED"),
            ("internal_continuation", False, "MISSION_BOUNDARY_INVALID"),
            ("scope_expansion", True, "MISSION_BOUNDARY_INVALID"),
            ("remote_publication", True, "MISSION_BOUNDARY_INVALID"),
            ("product_audit_base", "0" * 40, "HISTORICAL_AUDIT_BASE_DRIFT"),
            ("p3b_status", "IMPLEMENTATION_COMPLETE", "P3B_OVERCLAIM"),
            ("acceptance_boundary", "WHOLE_PRODUCT_PASS", "ACCEPTANCE_OVERCLAIM"),
            ("mission_completion", "COMPLETE", "STATE_SEMANTIC_DRIFT:mission_completion"),
            ("main_adoption", "ADOPTED", "STATE_SEMANTIC_DRIFT:main_adoption"),
            ("native_capture_activation", "ENABLED", "STATE_SEMANTIC_DRIFT:native_capture_activation"),
            ("chromium_deep_activation", "LIVE_QUALIFIED", "STATE_SEMANTIC_DRIFT:chromium_deep_activation"),
        ]:
            changed = copy.deepcopy(state); changed[key] = bad
            record("mission_" + key + "_drift_refused", lambda changed=changed,token=token: rejected(lambda: ps.state_check(changed), [token]))
        for defect_id in ("F-03", "F-05", "F-06"):
            changed = copy.deepcopy(state)
            next(d for d in changed["known_defects"] if d["id"] == defect_id)["verdict"] = "LIVE_QUALIFIED"
            record("mission_" + defect_id + "_overclaim_refused", lambda changed=changed: rejected(lambda: ps.state_check(changed), ["VERDICT_DRIFT"]))
        changed = copy.deepcopy(state)
        next(d for d in changed["known_defects"] if d["id"] == "F-02")["verdict"] = "SYNTHETIC_VERIFIED"
        record("mission_F-02_loaded_timeout_erasure_refused", lambda changed=changed: rejected(
            lambda: ps.state_check(changed), ["VERDICT_DRIFT:F-02"]))
        # Historical state remains separately valid; mission is not permission to relabel its audit.
        historical = json.loads(ps.git(repo, "show", "4e833988ed27086dd6d4acb78d39fc3e9c98c707:control/state.json"))
        record("historical_reset_state_preserved", lambda: ps.state_check(historical))
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
        record("legacy_appendix_in_current12_without_extra_source", lambda: check(
            inputs["LEGACY_SOURCE_INTEGRATION.md"] in (a / "payload" / ps.NAMES[11]).read_text(encoding="utf-8"),
            "source11 carries complete provenance and historical originals"))
        access = (a / "FRESH_CHAT_ACCESS_DIAGNOSTIC_PROMPT.md").read_text(encoding="utf-8")
        record("retained_history_not_current_inventory_failure", lambda: check(
            ps.HISTORY_POLICY in access and all(name in access for name in ps.RETAINED_HISTORY)
            and "버전 모호성" in access and "모두 직접 읽었을 때만 ACCESS_PASS" in access,
            "legacy eight permitted, exact current12 access and ambiguity gates retained"))
        record("deterministic_all_bytes_including_zip", lambda: check(inventory(a) == inventory(b), "two independent outputs identical"))
        record("exact_12_and_fresh_extraction", lambda: check(baseline["logical_sources"] == 12 and baseline["fresh_extraction"] == "PASS", "12 Sources and extraction verified"))
        if ps.mission_state(state):
            ledger = (a / "payload" / ps.NAMES[8]).read_text(encoding="utf-8")
            handoff = (a / "payload" / ps.NAMES[9]).read_text(encoding="utf-8")
            record("mission_continuation_separated_from_external_authority", lambda: check(
                "internal continuation: true" in ledger and "scope expansion: false" in ledger and
                "remote publication: false" in ledger and "별도 승인을 기다린다" not in handoff,
                "authorized internal work is distinct from scope expansion/publication"))
            manifest = ps.read_json(a / "PROJECT_SOURCE_MANIFEST.json")
            record("historical_audit_and_product_delta_separate", lambda: check(
                manifest["resolved_state"]["product_audit_base"] == ps.AUDIT_BASE and
                manifest["resolved_state"]["product_changes_since_historical_audit"] == ps.product_diff(repo, ps.AUDIT_BASE),
                "historical base preserved; committed product delta measured separately"))
        record("preview_refused_without_opt_in", lambda: rejected(lambda: ps.verify(a, repo, False, True), ["PREVIEW_REFUSED"]))
        record("existing_output_refused", lambda: rejected(lambda: ps.make(repo, a, True), ["OUTPUT_ALREADY_EXISTS"]))
        ident = ps.identity(repo)
        if state.get("source_package_policy") == "EXACT_12_OWNER_REQUESTED_CANDIDATE_OR_CANONICAL_SNAPSHOT":
            record("candidate_policy_accepts_exact_branch", lambda: ps.candidate_check(state, ident))
            no_authority = copy.deepcopy(state)
            no_authority["source_package_policy"] = "EXACT_12_MARKDOWN_REPLACEMENT_CANONICAL_MAIN_ONLY"
            record("candidate_missing_authority_refused", lambda: rejected(lambda: ps.candidate_check(no_authority, ident), ["CANDIDATE_SOURCE_NOT_AUTHORIZED"]))
            record("candidate_wrong_branch_refused", lambda: rejected(lambda: ps.candidate_check(state, {**ident, "branch": "main"}), ["CANDIDATE_BRANCH_NOT_AUTHORIZED"]))
            promoted = copy.deepcopy(state)
            promoted["mission_completion"] = "COMPLETE"
            record("candidate_completion_overclaim_refused", lambda: rejected(lambda: ps.candidate_check(promoted, ident), ["CANDIDATE_ACCEPTANCE_OVERCLAIM"]))
            record("candidate_preview_conflict_refused", lambda: rejected(lambda: ps.make(repo, temp / "conflict", True, True), ["EXPORT_MODE_CONFLICT"]))
            actual_git = ps.git
            def dirty_git(root, *args):
                return "?? unreviewed.txt" if args == ("status", "--porcelain") else actual_git(root, *args)
            with mock.patch.object(ps, "git", side_effect=dirty_git):
                record("candidate_untracked_refused", lambda: rejected(lambda: ps.make(repo, temp / "dirty-candidate", candidate=True), ["CANDIDATE_DIRTY"]))
            # Positive export is executed after the test/tool commit; do not mock
            # cleanliness to claim an actual source-bound candidate package.
            if not ps.git(repo, "status", "--porcelain"):
                candidate_result = ps.make(repo, temp / "candidate", candidate=True)
                record("candidate_actual_export_and_extraction", lambda: check(candidate_result["qualification"] == "OWNER_REQUESTED_CANDIDATE_SNAPSHOT" and candidate_result["fresh_extraction"] == "PASS", "actual clean candidate, exact12 extraction"))
                record("candidate_verify_without_preview_override", lambda: ps.verify(temp / "candidate", repo, extract=True))
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
