"""Fail closed on non-public email in reachable Git objects; never print matches.

Allow only GitHub noreply identities, the public Claude attribution address,
and reserved example fixture domains. The OpenAI URL-userinfo negative fixture
is allowed only in its existing source blob path. This is not a general secret scanner.
"""
import argparse
import re
import subprocess
import sys

EMAIL = re.compile(rb"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}")


def allowed(value, path=""):
    value = value.lower()
    domain = value.split(b"@")[-1]
    return (domain == b"users.noreply.github.com"
            or value == b"noreply@anthropic.com"
            or domain in (b"example.invalid", b"example.test", b"example.com")
            or (path == "src-tauri/src/device_login.rs" and domain == b"auth.openai.com"))


def git(*args):
    return subprocess.check_output(["git", *args])


def scan(refs):
    # Scan commit messages/identities, annotated tags and every historical blob,
    # including deleted files. Batch reading avoids one process per object.
    objects = git("rev-list", "--objects", *refs).splitlines()
    for ref in refs:
        objects.append(git("rev-parse", "--verify", ref).strip())
    unique = {}
    for item in objects:
        oid, _, path = item.partition(b" ")
        unique.setdefault(oid, path.decode("utf-8", "replace"))
    p = subprocess.Popen(["git", "cat-file", "--batch"], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    data, _ = p.communicate(b"\n".join(unique) + b"\n")
    if p.returncode:
        raise RuntimeError("Git object read failed")
    pos = 0
    failures = []
    for oid, path in unique.items():
        end = data.index(b"\n", pos)
        header = data[pos:end].split()
        if len(header) != 3:
            raise RuntimeError("Missing object")
        size = int(header[2]); pos = end + 1
        content = data[pos:pos + size]; pos += size + 1
        if header[1] not in (b"blob", b"commit", b"tag"):
            continue
        if any(not allowed(email, path) for email in EMAIL.findall(content)):
            failures.append(oid.decode())
    # Only opaque object IDs are logged; neither email nor source line is echoed.
    print(f"public-email: objects={len(unique)} blocked={len(failures)}")
    for oid in failures:
        print(f"blocked object {oid}")
    return 1 if failures else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--ref", action="append")
    parser.add_argument("--pre-push", action="store_true")
    args = parser.parse_args()
    refs = args.ref or []
    if args.pre_push:
        for line in sys.stdin:
            fields = line.split()
            if len(fields) != 4:
                raise SystemExit("Malformed pre-push input")
            if set(fields[1]) != {"0"}:
                refs.append(fields[1])
        if not refs:
            raise SystemExit(0)
    raise SystemExit(scan(refs or ["HEAD"]))
