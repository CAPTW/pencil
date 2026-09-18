#!/usr/bin/env python3
"""Deterministic, exact-inventory Project Source export. Python standard library only."""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import tempfile
import zipfile

NAMES = (
    "00_READ_ME_FIRST.md", "01_PRODUCT_CHARTER.md", "02_CURRENT_REPOSITORY_STATE.md",
    "03_ARCHITECTURE_PRIVACY_AND_THREAT_MODEL.md", "04_CURRENT_DECISIONS_AND_AUTHORITY.md",
    "05_PHASE_STEP_ROADMAP.md", "06_ACCEPTANCE_AND_TEST_MATRIX.md",
    "07_MULTI_AGENT_DEVELOPMENT_CONTRACT.md", "08_ACTIVE_TASK_LEDGER.md",
    "09_SESSION_HANDOFF.md", "10_CURRENT_REPOSITORY_AUDIT.md", "11_SOURCE_INDEX_AND_LIFECYCLE.md",
)
DOCS = {
    NAMES[1]: "PRODUCT_CHARTER.md", NAMES[3]: "ARCHITECTURE_PRIVACY.md",
    NAMES[4]: "CURRENT_DECISIONS.md", NAMES[5]: "ROADMAP.md",
    NAMES[6]: "ACCEPTANCE_MATRIX.md", NAMES[7]: "MULTI_AGENT_CONTRACT.md",
    NAMES[10]: "CURRENT_AUDIT.md", NAMES[11]: "SOURCE_LIFECYCLE.md",
}
META = {"PROJECT_SOURCE_MANIFEST.json", "SHA256SUMS.txt", "VERIFICATION_RECORD.json",
        "OWNER_HANDOFF.md", "SOURCE_REPLACEMENT_MATRIX.md", "FRESH_CHAT_ACCESS_DIAGNOSTIC_PROMPT.md",
        "FRESH_CHAT_SEMANTIC_VERIFICATION_PROMPT.md"}
GUARDED = ("src/", "src-tauri/", "benchmarks/")
GUARDED_FILES = {"package.json", "package-lock.json", "index.html", "tsconfig.json", "tsconfig.node.json", "vite.config.ts"}
SAFETY_TASK = "GRAMMAR-P3A-R1-CURRENT-HEAD-SAFETY-STABILIZATION-CONFIRMED-CANCEL-PROCESS-DRAFT-AND-APPLY-BOUNDARY-DEFECTS-BOUNDED-REPAIR"


def fail(condition, code):
    if not condition:
        raise ValueError(code)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def encode(obj):
    return (json.dumps(obj, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def read_json(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            fail(key not in result, f"DUPLICATE_JSON_KEY:{key}")
            result[key] = value
        return result
    return json.loads(path.read_text(encoding="utf-8-sig"), object_pairs_hook=unique)


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True, encoding="utf-8").strip()


def identity(root):
    return {"branch": git(root, "branch", "--show-current"), "head": git(root, "rev-parse", "HEAD"),
            "tree": git(root, "rev-parse", "HEAD^{tree}"), "parent": git(root, "rev-parse", "HEAD^"),
            "subject": git(root, "log", "-1", "--format=%s")}


def digest_files(files):
    return sha("\n".join(f"{name}\t{sha(files[name])}" for name in sorted(files)).encode("utf-8"))


def state_check(state):
    required = {"product_target", "p3a_status", "p3b_status", "first_adapter_decision", "active_phase",
                "active_step", "active_task", "exact_next_task", "acceptance_boundary", "provider_kinds",
                "known_defects", "automatic_continuation", "next_gate_executed", "d020_verdict", "product_audit_base"}
    fail(required <= state.keys(), "STATE_FIELDS_MISSING")
    fail(state["automatic_continuation"] is False and state["next_gate_executed"] is False, "NEXT_GATE_FORBIDDEN")
    fail(state["product_target"] == "P3A_PRESERVED_PLUS_P3B_PROACTIVE_INLINE_ASSIST", "TARGET_DRIFT")
    fail(state["p3b_status"] == "DIRECTION_FROZEN_NOT_IMPLEMENTED", "P3B_OVERCLAIM")
    fail(state["first_adapter_decision"] == "CHROMIUM_TEXTAREA_CONTENTEDITABLE_MV3", "ADAPTER_DRIFT")
    fail(state["provider_kinds"] == ["codex", "antigravity", "claude"], "PROVIDER_DRIFT")
    fail(state["acceptance_boundary"] == "P1_03_HISTORICAL_ONLY_NO_CURRENT_PRODUCT_ACCEPTANCE", "ACCEPTANCE_OVERCLAIM")
    fail({x["id"] for x in state["known_defects"]} == {f"F-0{i}" for i in range(1, 7)}, "VERDICT_SET_INVALID")
    safety = any(x["id"] in ("F-01", "F-02", "F-05", "F-06") and
                 x["verdict"] in ("CONFIRMED_CURRENT_DEFECT", "PARTIALLY_CONFIRMED") for x in state["known_defects"])
    if safety:
        fail(state["exact_next_task"] == SAFETY_TASK, "UNSAFE_NEXT_TASK")
    fail(state["d020_verdict"] != "D020_UNRESOLVED", "D020_UNRESOLVED")
    expected = {"p3a_status": "IMPLEMENTED_WITH_CURRENT_SAFETY_DEFECTS",
                "active_phase": "PHASE_0_AUTHORITY_RESET", "active_step": "COMPLETE",
                "active_task": "NONE", "next_task_status": "FROZEN_NOT_STARTED",
                "next_gate": "PHASE_1_P3A_STABILIZATION",
                "d020_verdict": "D020_DOCUMENTATION_OVERCLAIM", "exact_next_task": SAFETY_TASK}
    for key, value in expected.items():
        fail(state.get(key) == value, f"STATE_SEMANTIC_DRIFT:{key}")
    fail(len(state["known_defects"]) == 6, "DUPLICATE_VERDICT")
    for defect in state["known_defects"]:
        fail(defect["verdict"] == ("PARTIALLY_CONFIRMED" if defect["id"] == "F-03" else "CONFIRMED_CURRENT_DEFECT"),
             f"VERDICT_DRIFT:{defect['id']}")


def semantic_check(state, inputs):
    # Machine-readable normative declarations, plus independent prose review.
    # This deliberately does not pretend to understand arbitrary natural language.
    fields = {"CURRENT_PRODUCT_TARGET": "product_target", "P3A_STATUS": "p3a_status",
              "P3B_STATUS": "p3b_status", "FIRST_ADAPTER_DECISION": "first_adapter_decision",
              "EXACT_NEXT_TASK": "exact_next_task", "D020_VERDICT": "d020_verdict"}
    required = {"PRODUCT_CHARTER.md": {"CURRENT_PRODUCT_TARGET"},
                "FIRST_ADAPTER_DECISION.md": {"FIRST_ADAPTER_DECISION"},
                "ROADMAP.md": {"EXACT_NEXT_TASK"}}
    for name, content in inputs.items():
        found = set()
        for key, value in re.findall(r"(?m)^([A-Z][A-Z0-9_]*)=([^\r\n]+)$", content):
            if key in fields:
                fail(value.strip() == state[fields[key]], f"DOC_SEMANTIC_DRIFT:{name}:{key}")
                found.add(key)
        fail(required.get(name, set()) <= found, f"DOC_AUTHORITY_MARKER_MISSING:{name}")
        fail("P3B_IMPLEMENTATION_COMPLETE=true" not in content, "UNSUPPORTED_COMPLETION")
        for match in re.findall(r"GRAMMAR-P3[AB]-[A-Z0-9]+-[A-Z0-9-]+", content):
            if "STABILIZATION" in match or "DESIGN-FREEZE" in match:
                fail(match == state["exact_next_task"], f"DOC_NEXT_TASK_CONFLICT:{name}")
    fail(all(f"## PHASE {i} " in inputs["ROADMAP.md"] for i in range(11)), "ROADMAP_PHASE_MISSING")


def source_inputs(root):
    inputs = {}
    for filename in set(DOCS.values()) | {"FIRST_ADAPTER_DECISION.md"}:
        path = root / "docs" / "control" / filename
        fail(path.is_file(), f"DOC_MISSING:{filename}")
        inputs[filename] = path.read_text(encoding="utf-8-sig").replace("\r\n", "\n")
    return inputs


def render(state, ident, inputs, package_id, preview):
    # Git identity is measured, never committed as a self-referential current HEAD.
    resolved = copy.deepcopy(state)
    resolved.update(ident)
    resolved["package_id"] = package_id
    resolved["qualification"] = "PREVIEW_NOT_QUALIFIED" if preview else "CANONICAL_MAIN_SNAPSHOT"
    resolved["source_content_digest_location"] = "PROJECT_SOURCE_MANIFEST.json/resolved_state/source_content_digest"
    payload = {name: inputs[doc].encode("utf-8") for name, doc in DOCS.items()}
    payload[NAMES[0]] = ("# Grammar Project Source — 읽기 순서\n\n"
        "이 exact 12-file 세트는 기존 active Source 전체를 대체한다. 현재 사실은 02의 단일 machine-readable snapshot에 있다. "
        "Repository Git/current bytes > 실제 현재 실행 > 코드 > Repository 결정 > 이 Source > 과거 대화 순서로 확인한다.\n\n"
        "00 → 02 → 04 → 08 → 선택 작업 관련 Source 순서로 읽는다. P3-B 방향 승인과 구현 승인은 다르다. "
        "역사 acceptance는 현재 제품 수용이 아니다. 다음 작업은 02에 명시된 정확히 하나이며 자동 실행하지 않는다.\n\n"
        "본문의 repo 상대경로는 원 저장소에서 해결한다. Source만으로 못 여는 파일/실행은 미확인으로 남긴다. "
        "현재 선택/문서 원문이나 credentials를 요구하지 않는다.\n").encode("utf-8")
    payload[NAMES[2]] = ("# 현재 Repository snapshot — 단일 current-fact 권위\n\n"
        "아래 identity는 export 시 Git에서 측정했다. product_audit_base는 제품 코드가 감사된 역사 기준이며 "
        "current head와 혼동하지 않는다. source digest는 자기참조를 피하기 위해 payload 밖 manifest에 둔다.\n\n```json\n" +
        encode(resolved).decode("utf-8") + "```\n").encode("utf-8")
    payload[NAMES[4]] += ("\n\n" + inputs["FIRST_ADAPTER_DECISION.md"]).encode("utf-8")
    payload[NAMES[8]] = ("# Active task ledger\n\nCurrent fact는 02의 JSON만 사용한다.\n\n"
        f"- completed task: `{state.get('completed_task', '')}`\n"
        f"- active task: `{state['active_task']}`\n"
        f"- exact next task: `{state['exact_next_task']}`\n"
        "- next task status: FROZEN_NOT_STARTED\n- next gate executed: false\n- automatic continuation: false\n\n"
        "새 실행은 별도 Owner Task Packet이 필요하다. 이 Source 업로드 자체는 구현 실행 승인이 아니다.\n").encode("utf-8")
    payload[NAMES[9]] = ("# Session handoff\n\n02에서 measured Git identity와 상태를 읽고 10의 현재 결함 및 06의 검증 한계를 확인한다. "
        "05의 Phase-Step 경로와 07의 exact-base/worktree/path-claim 계약을 따른다.\n\n"
        "도구 전환 시 exact Task Packet, base/candidate identity, diff+hash, 완료/실패/미실행 검증, 가설/반증, "
        "allowed paths와 남은 단계/no-go를 전달한다. 이전 agent 결론을 그대로 신뢰하지 않는다.\n\n"
        f"허용 가능한 후속 검토 대상은 `{state['exact_next_task']}` 하나다. 아직 실행되지 않았으며 별도 승인을 기다린다.\n").encode("utf-8")
    return payload, resolved


def product_diff(root, base):
    paths = git(root, "diff", "--name-only", base, "HEAD").splitlines()
    existing = set(git(root, "ls-tree", "-r", "--name-only", base).splitlines())
    return [p for p in paths if p.startswith(GUARDED) or p in GUARDED_FILES
            or (p in existing and p.startswith("scripts/"))]


def canonical_check(root):
    # The owner-designated checkout is the primary, non-linked worktree.
    common = pathlib.Path(git(root, "rev-parse", "--path-format=absolute", "--git-common-dir")).resolve()
    fail(root.resolve() == common.parent and common.name == ".git", "CANONICAL_CHECKOUT_REQUIRED")
    fail(identity(root)["branch"] == "main", "CANONICAL_MAIN_REQUIRED")


def make(root, out, preview=False):
    fail(not out.exists(), "OUTPUT_ALREADY_EXISTS")
    fail(not out.is_relative_to(root), "OUTPUT_INSIDE_REPOSITORY")
    state = read_json(root / "control/state.json")
    state_check(state)
    ident = identity(root)
    if not preview:
        fail(ident["branch"] == "main", "CANONICAL_MAIN_REQUIRED")
        canonical_check(root)
        fail(not git(root, "diff", "--name-only") and not git(root, "diff", "--cached", "--name-only"), "TRACKED_DIRTY")
        fail(not product_diff(root, state["product_audit_base"]), "PRODUCT_DIFF_NONZERO")
    inputs = source_inputs(root)
    semantic_check(state, inputs)
    semantic = sha(encode(state) + encode(inputs))
    package_id = f"GRAMMAR-P3B-R0-{ident['head'][:12]}-{semantic[:12]}"
    payload, resolved = render(state, ident, inputs, package_id, preview)
    digest = digest_files(payload)
    resolved["source_content_digest"] = digest
    manifest = {"schema_version": 1, "package_id": package_id,
        "qualification": "PREVIEW_NOT_QUALIFIED" if preview else "CANONICAL_MAIN_SNAPSHOT",
        "logical_source_count": 12, "logical_source_names": list(NAMES),
        "resolved_state": resolved, "semantic_input_digest": semantic,
        "payload": {name: {"sha256": sha(data), "bytes": len(data)} for name, data in payload.items()},
        "source_documents": {k: sha(v.encode()) for k, v in inputs.items()},
        "digest_algorithm": "SHA256(sorted filename + TAB + SHA256(bytes), LF joined, no trailing LF)",
        "ui_application_state": "OUTSIDE_PAYLOAD_NOT_CURRENT_TRUTH"}
    handoff = ("# Owner handoff\n\n" + ("**PREVIEW — NOT QUALIFIED. 이 패키지는 적용하지 않는다.**\n\n" if preview else "") +
        "1. 현재 Grammar Project의 기존 active Sources를 전부 제거한다.\n"
        "2. payload/의 아래 exact 12 markdown만 업로드한다.\n"
        "3. ZIP은 업로드하지 않는다.\n4. manifest/checksums/verification record/handoff는 업로드하지 않는다.\n"
        "5. 중복 이름 Source를 만들지 않는다.\n6. genuinely fresh top-level chat에서 access diagnostic을 실행한다.\n"
        "7. access PASS 후 같은 fresh chat에서 semantic verification을 실행한다.\n"
        "8. 아래 expected values를 사용한다.\n9. 접근 누락은 해당 payload 파일만 재적용하고 중복은 제거한다. "
        "semantic mismatch는 수동 문장 패치 대신 canonical Repository에서 다시 생성한다.\n"
        "10. 성공 뒤에도 next task를 자동 실행하지 않는다. 별도 Owner 실행 권한을 부여한다.\n\n" +
        "\n".join(f"- {name}" for name in NAMES) + "\n\nExpected values:\n```json\n" +
        encode(resolved).decode() + "```\n")
    access = ("Genuinely fresh top-level chat에서만 실행. 다음 exact 12 Source를 각각 열고 실제 첫 heading과 "
        "읽은 내용 한 문장씩 보고하라. 이름 목록을 기억으로 복창하지 말라. 누락/중복/접근 불가는 "
        "ACCESS_FAIL로 멈추고 추측하지 말라. ZIP/manifest는 Source가 아니다. 모두 직접 읽었을 때만 ACCESS_PASS.\n\n" +
        "\n".join(NAMES) + "\n")
    source_snapshot = {key: value for key, value in resolved.items() if key != "source_content_digest"}
    semantic_prompt = ("같은 fresh chat에서 ACCESS_PASS 이후에만 실행. 02 JSON에서 아래 expected 값을 읽어 비교하고, "
        "04 첫 adapter, 05 roadmap, 08 exact next task, 10 F01-F06/D020 판정과 06 검증 한계의 일관성을 확인하라. "
        "P3-B 구현/현재 제품 acceptance/다음 task 실행을 선언하지 말라. 불일치는 SEMANTIC_FAIL, 모두 직접 확인했을 때만 "
        "SEMANTIC_PASS. source_content_digest 실제 값은 payload 밖 manifest의 Owner 검증 대상이므로 "
        "이 채팅의 접근 성공 조건에 넣지 않는다. digest 위치 필드는 02와 비교한다. 어떠한 작업도 실행하지 말라.\n\n```json\n" + encode(source_snapshot).decode() + "```\n")
    files = {f"payload/{k}": v for k, v in payload.items()}
    files.update({"PROJECT_SOURCE_MANIFEST.json": encode(manifest), "OWNER_HANDOFF.md": handoff.encode(),
        "FRESH_CHAT_ACCESS_DIAGNOSTIC_PROMPT.md": access.encode(), "FRESH_CHAT_SEMANTIC_VERIFICATION_PROMPT.md": semantic_prompt.encode(),
        "SOURCE_REPLACEMENT_MATRIX.md": ("# Full replacement matrix\n\n"
            "기존 active Source 전체를 아래 exact 12개로 교체한다. 중복 bootstrap/handoff/current-state는 통합하고 과거 gate는 역사 기록으로만 취급한다.\n\n"
            "| New logical Source | Repository source / role |\n|---|---|\n" +
            "\n".join(f"| {n} | {DOCS.get(n, 'generated from control/state.json')} |" for n in NAMES) +
            "\n\nD020은 CURRENT_DECISIONS의 정정 판정을 사용한다. 구 ZIP을 새 active Source로 남기지 않는다.\n").encode("utf-8"),
        "VERIFICATION_RECORD.json": encode({"schema_version": 1, "package_id": package_id,
            "status": "LOCAL_GENERATOR_CHECK_PENDING", "product_acceptance": "NOT_DECLARED",
            "fresh_chat_verification": "OWNER_PROCEDURE_NOT_EXECUTED", "independent_review": "EXTERNAL_EVIDENCE_REQUIRED"})})
    out.mkdir(parents=True, exist_ok=False)
    for name, data in files.items():
        path = out / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    # Check generated bytes first. Record does not claim independent or UI success.
    check_core(out, root, preview)
    files["VERIFICATION_RECORD.json"] = encode({"schema_version": 1, "package_id": package_id,
        "status": "PASS_LOCAL_GENERATOR", "product_acceptance": "NOT_DECLARED",
        "fresh_chat_verification": "OWNER_PROCEDURE_NOT_EXECUTED", "independent_review": "EXTERNAL_EVIDENCE_REQUIRED"})
    (out / "VERIFICATION_RECORD.json").write_bytes(files["VERIFICATION_RECORD.json"])
    sums = "".join(f"{sha(data)}  {name}\n" for name, data in sorted(files.items()))
    (out / "SHA256SUMS.txt").write_text(sums, encoding="utf-8", newline="\n")
    with zipfile.ZipFile(out / "transport.zip", "x", zipfile.ZIP_DEFLATED) as archive:
        for name in sorted(set(files) | {"SHA256SUMS.txt"}):
            entry = zipfile.ZipInfo(name, date_time=(2020, 1, 1, 0, 0, 0))
            entry.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(entry, (out / name).read_bytes())
    return verify(out, root, preview, extract=True)


def check_core(out, root, allow_preview):
    m = read_json(out / "PROJECT_SOURCE_MANIFEST.json")
    fail(m["qualification"] in {"PREVIEW_NOT_QUALIFIED", "CANONICAL_MAIN_SNAPSHOT"}, "QUALIFICATION_INVALID")
    fail(m["logical_source_count"] == 12 and m["logical_source_names"] == list(NAMES), "LOGICAL_INVENTORY")
    names = sorted(p.name for p in (out / "payload").iterdir())
    fail(names == sorted(NAMES), "PAYLOAD_EXACT_SET")
    fail(not any(p.is_symlink() or not p.is_file() for p in (out / "payload").iterdir()), "PAYLOAD_NOT_PLAIN_FILES")
    fail(allow_preview or m["qualification"] == "CANONICAL_MAIN_SNAPSHOT", "PREVIEW_REFUSED")
    payload = {n: (out / "payload" / n).read_bytes() for n in NAMES}
    fail(set(m["payload"]) == set(NAMES), "MANIFEST_PAYLOAD_SET")
    for n, data in payload.items():
        fail(m["payload"][n] == {"sha256": sha(data), "bytes": len(data)}, f"PAYLOAD_HASH:{n}")
    resolved = m["resolved_state"]
    fail(resolved["qualification"] == m["qualification"] and resolved["package_id"] == m["package_id"], "MANIFEST_IDENTITY_CONFLICT")
    state_check(resolved)
    fail(resolved["source_content_digest"] == digest_files(payload), "PAYLOAD_DIGEST")
    snapshot = payload[NAMES[2]].decode("utf-8").split("```json\n", 1)[1].split("```", 1)[0]
    snapshot = json.loads(snapshot)
    fail(snapshot == {k: v for k, v in resolved.items() if k != "source_content_digest"}, "SNAPSHOT_CONTRADICTION")
    text = "\n".join(data.decode("utf-8") for data in payload.values())
    for forbidden in ("Owner manual application pending", "fresh chat verification pending", "Project Source UI not yet applied"):
        fail(forbidden.lower() not in text.lower(), "VOLATILE_UI_CLAIM")
    fail("P3B_IMPLEMENTATION_COMPLETE=true" not in text, "UNSUPPORTED_COMPLETION")
    if root:
        current = identity(root)
        fail(all(resolved[k] == v for k, v in current.items()), "GIT_IDENTITY_STALE")
        state = read_json(root / "control/state.json")
        inputs = source_inputs(root)
        semantic_check(state, inputs)
        fail(sha(encode(state) + encode(inputs)) == m["semantic_input_digest"], "INPUT_SEMANTICS_STALE")
        expected, _ = render(state, current, inputs, m["package_id"], m["qualification"] == "PREVIEW_NOT_QUALIFIED")
        fail(expected == payload, "RENDER_CONTRADICTION")
        fail(m["source_documents"] == {k: sha(v.encode()) for k, v in inputs.items()}, "DOC_DIGEST_DRIFT")
        if m["qualification"] == "CANONICAL_MAIN_SNAPSHOT":
            fail(current["branch"] == "main", "CANONICAL_MAIN_REQUIRED")
            canonical_check(root)
            fail(not git(root, "diff", "--name-only") and not git(root, "diff", "--cached", "--name-only"), "TRACKED_DIRTY")
            fail(not product_diff(root, state["product_audit_base"]), "PRODUCT_DIFF_NONZERO")
    fail(m["package_id"] == f"GRAMMAR-P3B-R0-{resolved['head'][:12]}-{m['semantic_input_digest'][:12]}", "PACKAGE_ID_INVALID")
    return m


def verify(out, root=None, allow_preview=False, extract=False):
    m = check_core(out, root, allow_preview)
    inventory = {p.relative_to(out).as_posix() for p in out.rglob("*") if p.is_file()}
    expected = {f"payload/{n}" for n in NAMES} | META | {"transport.zip"}
    fail(inventory == expected, "PACKAGE_EXACT_SET")
    sums = {}
    for line in (out / "SHA256SUMS.txt").read_text(encoding="utf-8").splitlines():
        h, name = line.split("  ", 1)
        fail(name not in sums and name in expected and name not in {"SHA256SUMS.txt", "transport.zip"}, "CHECKSUM_PATH_INVALID")
        fail(re.fullmatch("[0-9a-f]{64}", h) is not None and sha((out / name).read_bytes()) == h, f"CHECKSUM_MISMATCH:{name}")
        sums[name] = h
    fail(set(sums) == expected - {"SHA256SUMS.txt", "transport.zip"}, "CHECKSUM_COVERAGE")
    record = read_json(out / "VERIFICATION_RECORD.json")
    fail(record["status"] == "PASS_LOCAL_GENERATOR" and record["package_id"] == m["package_id"], "VERIFICATION_RECORD")
    extraction = "NOT_RUN"
    if extract:
        with tempfile.TemporaryDirectory(prefix="grammar-source-verify-") as tmp:
            target = pathlib.Path(tmp)
            with zipfile.ZipFile(out / "transport.zip") as archive:
                fail(len(archive.namelist()) == len(set(archive.namelist())), "ZIP_DUPLICATE")
                fail(set(archive.namelist()) == expected - {"transport.zip"}, "ZIP_INVENTORY")
                for name in archive.namelist():
                    fail(not pathlib.PurePosixPath(name).is_absolute() and ".." not in pathlib.PurePosixPath(name).parts, "ZIP_TRAVERSAL")
                    fail(archive.read(name) == (out / name).read_bytes(), f"ZIP_BYTES:{name}")
                archive.extractall(target)
            check_core(target, root, allow_preview)
            fail(all((target / n).read_bytes() == (out / n).read_bytes() for n in expected - {"transport.zip"}), "EXTRACTION_BYTES")
            extraction = "PASS"
    return {"status": "PASS", "qualification": m["qualification"], "package_id": m["package_id"],
            "head": m["resolved_state"]["head"], "tree": m["resolved_state"]["tree"],
            "logical_sources": 12, "payload_digest": m["resolved_state"]["source_content_digest"],
            "manifest_sha256": sha((out / "PROJECT_SOURCE_MANIFEST.json").read_bytes()),
            "zip_sha256": sha((out / "transport.zip").read_bytes()), "fresh_extraction": extraction,
            "next_gate_executed": False, "automatic_continuation": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    gen = sub.add_parser("generate")
    gen.add_argument("--repo", type=pathlib.Path, required=True)
    gen.add_argument("--output", type=pathlib.Path, required=True)
    gen.add_argument("--preview", action="store_true")
    val = sub.add_parser("verify")
    val.add_argument("--package", type=pathlib.Path, required=True)
    val.add_argument("--repo", type=pathlib.Path)
    val.add_argument("--allow-preview", action="store_true")
    val.add_argument("--extract", action="store_true")
    args = parser.parse_args()
    try:
        root = args.repo.resolve() if args.repo else None
        result = make(root, args.output.resolve(), args.preview) if args.command == "generate" else verify(args.package.resolve(), root, args.allow_preview, args.extract)
        print(json.dumps(result, ensure_ascii=False, indent=2))
    except (ValueError, KeyError, OSError, subprocess.SubprocessError, zipfile.BadZipFile) as exc:
        print(json.dumps({"status": "FAIL", "error": str(exc)}, ensure_ascii=False))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
