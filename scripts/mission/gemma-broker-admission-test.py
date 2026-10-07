"""Owned binary startup rejection tests; never supplies a valid model."""
import hashlib
import json
import pathlib
import subprocess
import sys
import uuid

binary = pathlib.Path(sys.argv[1]).resolve(strict=True)
receipt = pathlib.Path(sys.argv[2]).resolve()
if receipt.exists():
    raise SystemExit("preserve previous receipt")
model = receipt.parent / ("owned-invalid-model-" + uuid.uuid4().hex + ".gguf")
model.write_bytes(b"synthetic invalid model")
image_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
model_hash = hashlib.sha256(model.read_bytes()).hexdigest()
base = [str(binary), image_hash, str(model), model_hash, str(binary), image_hash]
rows = []
try:
    for case, index, expected in [
        ("runtime_hash", 1, "gemma_image_hash"),
        ("host_hash", 5, "gemma_image_hash"),
        ("model_hash", 3, "gemma_model_hash"),
        ("control_eof", None, "gemma_broker_startup_revoked_or_deadline"),
    ]:
        args = base.copy()
        if index is not None:
            args[index] = "0" * 64
        process = subprocess.Popen([str(binary), *args], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            if index is None:
                process.stdin.close()
            code = process.wait(timeout=10)
            out = process.stdout.read()
            error = process.stderr.read().decode().strip()
            passed = code == 1 and not out and error == expected
            rows.append({"case": case, "exit": code, "error": error, "pass": passed})
        finally:
            if process.poll() is None:
                process.kill()  # Only this test's retained process handle.
                process.wait(timeout=5)
            for stream in (process.stdin, process.stdout, process.stderr):
                stream.close()
finally:
    model.unlink()
    receipt.write_text(json.dumps({"binarySha256": image_hash, "rows": rows}, indent=2))
assert len(rows) == 4 and all(row["pass"] for row in rows), rows
print("4 startup rejection checks passed")
