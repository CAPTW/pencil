#!/usr/bin/env python3
"""Small fail-closed governance validator. Python standard library only."""
import argparse
import copy
import datetime as dt
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path


class Invalid(ValueError):
    pass


def require(ok, message):
    if not ok:
        raise Invalid(message)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON field: " + key)
        result[key] = value
    return result


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"), object_pairs_hook=unique_object)


def validate(value, schema, schema_dir, at="$"):
    """Validate the deliberately small JSON Schema subset used by this repository."""
    if "$ref" in schema:
        return validate(value, read(schema_dir / schema["$ref"]), schema_dir, at)
    types = schema.get("type", [])
    types = [types] if isinstance(types, str) else types
    checks = {"object": lambda x: isinstance(x, dict), "array": lambda x: isinstance(x, list),
              "string": lambda x: isinstance(x, str), "boolean": lambda x: type(x) is bool,
              "null": lambda x: x is None, "integer": lambda x: type(x) is int}
    require(all(t in checks for t in types), at + ": unknown schema type")
    require(not types or any(checks[t](value) for t in types), at + ": invalid type")
    if "const" in schema:
        require(type(value) is type(schema["const"]) and value == schema["const"], at + ": invalid constant")
    if "enum" in schema:
        require(value in schema["enum"], at + ": invalid enum")
    if isinstance(value, str):
        require(len(value) >= schema.get("minLength", 0), at + ": empty string")
        require("pattern" not in schema or re.fullmatch(schema["pattern"], value), at + ": invalid pattern")
    if isinstance(value, list):
        require(len(value) >= schema.get("minItems", 0), at + ": empty list")
        for i, item in enumerate(value):
            validate(item, schema.get("items", {}), schema_dir, f"{at}[{i}]")
    if isinstance(value, dict):
        require(set(schema.get("required", [])) <= value.keys(), at + ": missing required fields")
        props = schema.get("properties", {})
        require(schema.get("additionalProperties", True) or value.keys() <= props.keys(), at + ": unknown fields")
        for key, item in value.items():
            if key in props:
                validate(item, props[key], schema_dir, at + "." + key)


def path_pattern(pattern):
    require(isinstance(pattern, str) and bool(pattern), "empty path scope")
    p = pattern.replace("\\", "/").lower()
    require(not p.startswith("/") and not p.endswith("/") and not re.search(r"[:\[\]{}!\x00-\x1f]", p), "unsupported/absolute path scope: " + pattern)
    require(all(part not in ("", ".", "..") and not part.endswith((" ", ".")) for part in p.split("/")), "unsafe path scope: " + pattern)
    require("***" not in p, "ambiguous glob: " + pattern)
    return p


def automaton(pattern):
    # ** spans separators; **/ can also match zero directories. Single * and ? cannot.
    p = path_pattern(pattern)
    tokens = re.findall(r"\*\*/|\*\*|\*|\?|[^*?]", p)
    def eps(i):
        return [i + 1] if i < len(tokens) and tokens[i] in ("*", "**", "**/") else []
    def edges(i):
        if i == len(tokens):
            return []
        t = tokens[i]
        if t in ("*", "**", "**/"):
            return [("NONSLASH" if t == "*" else "ANY", i)]
        return [("NONSLASH" if t == "?" else t, i + 1)]
    return len(tokens), eps, edges


def overlaps(left, right):
    # Product-NFA emptiness test. **/ intentionally overapproximates for safe exclusion.
    n, ea, ta = automaton(left)
    m, eb, tb = automaton(right)
    todo, seen = [(0, 0)], set()
    def compatible(a, b):
        if a == "ANY" or b == "ANY":
            return True
        if a == "NONSLASH":
            return b != "/"
        if b == "NONSLASH":
            return a != "/"
        return a == b
    while todo:
        a, b = todo.pop()
        if (a, b) in seen:
            continue
        seen.add((a, b))
        if (a, b) == (n, m):
            return True
        todo.extend((x, b) for x in ea(a))
        todo.extend((a, x) for x in eb(b))
        todo.extend((x, y) for ca, x in ta(a) for cb, y in tb(b) if compatible(ca, cb))
    return False


def matches(pattern, concrete):
    p, c = path_pattern(pattern), path_pattern(concrete)
    require(not any(x in c for x in "*?"), "changed path must be concrete")
    out, i = "", 0
    while i < len(p):
        if p[i:i+3] == "**/":
            out += "(?:.*/)?"; i += 3
        elif p[i:i+2] == "**":
            out += ".*"; i += 2
        elif p[i] == "*":
            out += "[^/]*"; i += 1
        elif p[i] == "?":
            out += "[^/]"; i += 1
        else:
            out += re.escape(p[i]); i += 1
    return re.fullmatch(out, c) is not None


def scopes(task):
    require(bool(task["authorized_paths"]), "no authorized scope")
    require(task["next_gate_policy"] == "DO_NOT_EXECUTE_NEXT_GATE", "task cannot authorize next gate")
    for p in task["authorized_paths"] + task["forbidden_paths"] + task["required_reads"]:
        path_pattern(p)
    for a in task["authorized_paths"]:
        for f in task["forbidden_paths"]:
            require(not overlaps(a, f), "authorized/forbidden overlap: " + a + " <> " + f)


def timestamp(value):
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except (ValueError, TypeError, AttributeError) as exc:
        raise Invalid("invalid ISO timestamp") from exc
    require(parsed.tzinfo is not None, "timestamp requires timezone")
    return parsed


def claims_check(ledger, schema_dir, now=None):
    require(isinstance(ledger, dict) and set(ledger) == {"format_version", "claims"}, "invalid claim ledger keys")
    require(type(ledger["format_version"]) is int and ledger["format_version"] == 1, "claim format version")
    require(isinstance(ledger["claims"], list), "claims must be a list")
    now = now or dt.datetime.now(dt.timezone.utc)
    active = []
    for c in ledger["claims"]:
        validate(c, read(schema_dir / "path-claim.schema.json"), schema_dir)
        created = timestamp(c["created_at"])
        expiry = timestamp(c["expires_at"]) if c["expires_at"] is not None else None
        require(expiry is None or expiry > created, "expiry must follow creation")
        for i, p in enumerate(c["claimed_paths"]):
            path_pattern(p)
            require(not any(overlaps(p, q) for q in c["claimed_paths"][:i]), "overlapping scopes within claim")
        if c["release_state"] == "active" and (expiry is None or expiry > now):
            require(created <= now, "active claim created in future")
            require(c["conflicting_claim"] is None, "unresolved conflicting claim")
            for old in active:
                require(not any(overlaps(a, b) for a in c["claimed_paths"] for b in old["claimed_paths"]), "active path claim overlap")
            active.append(c)
    return active


def git(root, *args):
    run = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True, check=False)
    require(run.returncode == 0, "git failed: " + run.stderr.strip())
    return run.stdout.strip()


def main_identity(root):
    return {"head": git(root, "rev-parse", "refs/heads/main"),
            "tree": git(root, "rev-parse", "refs/heads/main^{tree}"),
            "subject": git(root, "show", "-s", "--format=%s", "refs/heads/main")}


def base_check(task, identity):
    require(task["packet_status"] == "AUTHORIZED", "EXAMPLE_NOT_AUTHORIZED cannot execute")
    require(not any(x in task["owner_decision"].upper() for x in ("EXAMPLE", "NOT_AUTHORIZED")), "non-authorizing Owner decision")
    require(task["base_branch"] == "main", "canonical base must be main")
    for field in ("head", "tree", "subject"):
        require(task["base_" + field] == identity[field], "current main base " + field + " mismatch")


def state_check(state):
    require(state.get("automatic_continuation") is False, "automatic continuation must be false")
    require(state.get("next_gate_executed") is False, "next gate must not execute")
    require(isinstance(state.get("exact_next_task"), str) and bool(state["exact_next_task"]), "one exact next task required")
    defects = state.get("known_defects")
    require(isinstance(defects, list), "known defects list required")
    safety = any(d.get("verdict") in ("CONFIRMED_CURRENT_DEFECT", "PARTIALLY_CONFIRMED") for d in defects)
    if safety:
        require(state.get("next_gate") == "PHASE_1_P3A_STABILIZATION" and state["exact_next_task"].startswith("GRAMMAR-P3A-R1-"), "incompatible next gate for current safety defects")
    else:
        require(state.get("next_gate") == "P3B_CORE_DOCUMENT_MODEL_DESIGN_FREEZE" and state["exact_next_task"].startswith("GRAMMAR-P3B-P1-"), "incompatible next gate without current safety defects")


def wrapper_check(root):
    data = (root / "docs/control/MULTI_AGENT_CONTRACT.md").read_bytes().decode("utf-8").replace("\r\n", "\n").encode("utf-8")
    digest = hashlib.sha256(data).hexdigest()
    version = re.search(rb"^Contract-Version: (.+)$", data, re.M)
    require(version is not None, "contract version missing")
    for rel in ("AGENTS.md", "CLAUDE.md", ".cursor/rules/grammar-project.mdc"):
        text = (root / rel).read_text(encoding="utf-8").replace("\r\n", "\n")
        for key, expected in (("Canonical-Contract", "docs/control/MULTI_AGENT_CONTRACT.md"), ("Contract-Version", version[1].decode()), ("Contract-SHA256", digest)):
            found = re.findall(r"^" + key + r": (.+)$", text, re.M)
            require(found == [expected], "wrapper digest/version/source drift: " + rel)


def handoff_check(handoff):
    t = handoff["task_packet"]
    require(handoff["base_head"] == t["base_head"] and handoff["base_tree"] == t["base_tree"], "handoff base mismatch")
    require(handoff["allowed_paths"] == t["authorized_paths"], "handoff scope mismatch")
    scopes(t)


def result_check(root, result, task):
    require(result["packet_status"] == "AUTHORIZED", "example result cannot integrate")
    require(result["task_id"] == task["task_id"], "result task mismatch")
    require(result["entry_head"] == task["base_head"] and result["entry_tree"] == task["base_tree"], "result entry mismatch")
    head = result["candidate_head"]
    require(git(root, "rev-parse", head + "^{tree}") == result["candidate_tree"], "candidate tree mismatch")
    require(git(root, "rev-parse", head + "^") == result["candidate_parent"], "candidate parent mismatch")
    require(subprocess.run(["git", "-C", str(root), "merge-base", "--is-ancestor", task["base_head"], head], capture_output=True).returncode == 0, "candidate not descended from base")
    actual = git(root, "diff", "--name-only", "--no-renames", task["base_head"], head).splitlines()
    require(sorted(actual) == sorted(result["changed_paths"]), "actual changed paths differ from result")
    require(not result["unintended_paths"], "unintended paths block integration")
    for p in actual:
        require(any(matches(a, p) for a in task["authorized_paths"]) and not any(matches(f, p) for f in task["forbidden_paths"]), "changed path outside task scope: " + p)
    subjects = git(root, "log", "--reverse", "--format=%s", task["base_head"] + ".." + head).splitlines()
    require(subjects == result["commit_subjects"], "commit subjects mismatch")


def self_test(root):
    sd = root / "control/schemas"
    example = read(root / "control/examples/task.json")
    passed = []
    def rejects(name, fn):
        try:
            fn()
        except Invalid:
            passed.append(name)
        else:
            raise Invalid("negative test unexpectedly accepted: " + name)
    for kind in ("task", "result", "handoff"):
        packet = read(root / "control/examples" / (kind + ".json"))
        packet_schema = read(sd / (kind + ".schema.json"))
        for field in packet_schema["required"]:
            bad = copy.deepcopy(packet); del bad[field]
            rejects(kind + " missing " + field, lambda bad=bad, packet_schema=packet_schema: validate(bad, packet_schema, sd))
    rejects("duplicate JSON field", lambda: json.loads('{"automatic_continuation": true, "automatic_continuation": false}', object_pairs_hook=unique_object))
    extra = copy.deepcopy(example); extra["unexpected"] = True
    rejects("unknown field", lambda: validate(extra, read(sd / "task.schema.json"), sd))
    typed = copy.deepcopy(example); typed["authorized_paths"] = "docs/**"
    rejects("invalid field type", lambda: validate(typed, read(sd / "task.schema.json"), sd))
    bad2 = copy.deepcopy(example); bad2["automatic_continuation"] = True
    rejects("automatic true", lambda: validate(bad2, read(sd / "task.schema.json"), sd))
    bad3 = copy.deepcopy(example); bad3["base_head"] = "not-a-sha"
    rejects("invalid SHA", lambda: validate(bad3, read(sd / "task.schema.json"), sd))
    bad4 = copy.deepcopy(example); bad4["authorized_paths"] = ["SRC/**"]; bad4["forbidden_paths"] = ["src/App.tsx"]
    rejects("forbidden glob overlap", lambda: scopes(bad4))
    for p in ("../src/a", "C:/src/a", "//server/share", "src/[abc].ts", "src/../a", "src./a"):
        rejects("invalid scope " + p, lambda p=p: path_pattern(p))
    require(overlaps("SRC/**", "src/app.tsx") and overlaps("docs/*/x.md", "docs/a/*.md") and not overlaps("docs/**", "src/**"), "glob intersection self-test failed")
    require(matches("docs/**/*.md", "docs/a.md") and not matches("docs/*.md", "docs/x/a.md"), "glob matcher self-test failed")
    c = {"task_id":"a", "agent_id":"one", "base_head":example["base_head"], "claimed_paths":["src/**"], "claim_type":"exclusive", "created_at":"2020-01-01T00:00:00Z", "expires_at":None, "release_state":"active", "conflicting_claim":None, "owner_override":None}
    d = copy.deepcopy(c); d.update(task_id="b",agent_id="two",claimed_paths=["SRC/App.tsx"])
    rejects("active claim collision", lambda: claims_check({"format_version":1,"claims":[c,d]},sd))
    duplicated = copy.deepcopy(c); duplicated["claimed_paths"].append("SRC/App.tsx")
    rejects("overlap within claim", lambda: claims_check({"format_version":1,"claims":[duplicated]},sd))
    for field in read(sd / "path-claim.schema.json")["required"]:
        incomplete = copy.deepcopy(c); del incomplete[field]
        rejects("claim missing " + field, lambda incomplete=incomplete: claims_check({"format_version":1,"claims":[incomplete]},sd))
    d["release_state"] = "released"
    require(len(claims_check({"format_version":1,"claims":[c,d]},sd)) == 1, "released claim should not hold ownership")
    d["release_state"] = "active"; d["expires_at"] = "2020-01-02T00:00:00Z"
    require(len(claims_check({"format_version":1,"claims":[c,d]},sd)) == 1, "expired claim should not hold ownership")
    auth = copy.deepcopy(example); auth["packet_status"] = "AUTHORIZED"; auth["owner_decision"] = "TEST_ONLY_OWNER_DECISION"
    ident = {"head":"0"*40,"tree":auth["base_tree"],"subject":auth["base_subject"]}
    rejects("stale runtime base", lambda: base_check(auth, ident))
    rejects("nonexecutable example", lambda: base_check(example, ident))
    for field in ("tree", "subject"):
        mismatch = {"head":auth["base_head"],"tree":auth["base_tree"],"subject":auth["base_subject"]}
        mismatch[field] = "mismatch"
        rejects("stale runtime " + field, lambda mismatch=mismatch: base_check(auth, mismatch))
    result_example = read(root / "control/examples/result.json")
    result_example["next_gate_executed"] = True
    rejects("result next gate true", lambda: validate(result_example, read(sd / "result.schema.json"), sd))
    handoff_example = read(root / "control/examples/handoff.json")
    handoff_example["base_head"] = "0"*40
    rejects("handoff identity mismatch", lambda: handoff_check(handoff_example))
    st = {"automatic_continuation":False,"next_gate_executed":False,"exact_next_task":"GRAMMAR-P3B-P1-WRONG","next_gate":"P3B_CORE_DOCUMENT_MODEL_DESIGN_FREEZE","known_defects":[{"verdict":"CONFIRMED_CURRENT_DEFECT"}]}
    rejects("incompatible next gate", lambda: state_check(st))
    # Test real pin comparison with in-memory path stand-ins, without writing fixtures.
    class FakePath:
        def __init__(self, rel=""): self.rel = rel
        def __truediv__(self, rel): return FakePath(rel)
        def read_bytes(self): return b"Contract-Version: 1.0.0\n"
        def read_text(self, **kw): return "Canonical-Contract: docs/control/MULTI_AGENT_CONTRACT.md\nContract-Version: 1.0.0\nContract-SHA256: " + "0"*64 + "\n"
    rejects("wrapper digest drift", lambda: wrapper_check(FakePath()))
    canonical_text = "Contract-Version: 1.0.0\nCanonical content.\n"
    canonical_digest = hashlib.sha256(canonical_text.encode("utf-8")).hexdigest()
    class LineEndingPath:
        def __init__(self, changed=False): self.changed = changed
        def __truediv__(self, rel): return self
        def read_bytes(self):
            return (canonical_text + ("Content drift.\n" if self.changed else "")).replace("\n", "\r\n").encode("utf-8")
        def read_text(self, **kw):
            return ("Canonical-Contract: docs/control/MULTI_AGENT_CONTRACT.md\nContract-Version: 1.0.0\nContract-SHA256: " + canonical_digest + "\n").replace("\n", "\r\n")
    wrapper_check(LineEndingPath())
    rejects("true content drift with CRLF", lambda: wrapper_check(LineEndingPath(changed=True)))
    return passed


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    ap.add_argument("--task", type=Path)
    ap.add_argument("--result", type=Path)
    ap.add_argument("--execution", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    root = args.root.resolve(); sd = root / "control/schemas"
    try:
        wrapper_check(root)
        for name in ("task", "result", "handoff"):
            item = read(root / "control/examples" / (name + ".json"))
            validate(item, read(sd / (name + ".schema.json")), sd)
            if name == "handoff": handoff_check(item)
            elif name == "task": scopes(item)
            require((item.get("task_packet") or item)["packet_status"] == "EXAMPLE_NOT_AUTHORIZED", "example must be nonexecutable")
        active = claims_check(read(root / "control/path-claims.json"), sd)
        state_check(read(root / "control/state.json"))
        require(not args.execution or args.task is not None, "execution requires task packet")
        require(not args.result or args.task is not None, "result requires task packet")
        if args.task:
            task = read(args.task)
            validate(task, read(sd / "task.schema.json"), sd); scopes(task)
            if args.execution:
                base_check(task, main_identity(root))
                own = [c for c in active if c["task_id"] == task["task_id"] and c["base_head"] == task["base_head"]]
                require(bool(own) and all(any(path_pattern(p) == path_pattern(q) for c in own for q in c["claimed_paths"]) for p in task["authorized_paths"]), "execution requires exact active exclusive claims for authorized scopes")
            if args.result:
                result = read(args.result); validate(result, read(sd / "result.schema.json"), sd)
                if args.execution: result_check(root, result, task)
        tests = self_test(root) if args.self_test else []
        print(json.dumps({"status":"PASS", "execution_checked":args.execution, "negative_tests_passed":tests}, indent=2))
        return 0
    except (Invalid, OSError, ValueError, KeyError, TypeError) as exc:
        print("FAIL_GOVERNANCE: " + str(exc), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
