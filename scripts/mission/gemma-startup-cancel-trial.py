"""Owned GPU trial; requires development-only psutil (verified with 7.2.1).

Run sequentially: python scripts/mission/gemma-startup-cancel-trial.py health UNIQUE_LABEL
or warmup. No normal profile, registry, clipboard, document or cloud access.
6s test budget = existing 5s cleanup + 1s observation/process-exit allowance;
this is not the recommendation/Apply deadline or a new product cleanup budget.
"""
import sys, pathlib, subprocess, hashlib, time, threading, json, re, psutil

root = pathlib.Path('D:/dev/grammar-evidence/grammar-gemma-20261007')
repo = pathlib.Path(__file__).resolve().parents[2]
phase, label = sys.argv[1:]
assert phase in ('health', 'warmup') and re.fullmatch(r'[a-zA-Z0-9_-]+', label)
receipt = root / (label + '.json')
assert not receipt.exists()
binary = repo / 'src-tauri/target/release'
host = binary / 'grammar-chromium-host.exe'
broker = binary / 'grammar-gemma-broker.exe'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
report = dict(scope='OWNED_REAL_GPU_STARTUP_CANCEL', phase=phase,
              scriptSha256=sha(pathlib.Path(__file__)),
              brokerSha256=sha(broker), hostSha256=sha(host),
              budgetMs=6000, cleanupBudgetMs=5000, phases=[], children=[])
args = [str(broker), str(root/'runtime/llama-server.exe'),
        'a812655fed77b9f98d918e7aa0b1552403b76dacbeb9fa24c17dac9ed3d08ec1',
        str(root/'gemma-4-12b-it-qat-q4_0.gguf'),
        '93567e57a8fe10b23569b9d9ec38cd005deedf71e29477c421a4b83f418a538b',
        str(host), report['hostSha256']]
p = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                     stderr=subprocess.PIPE, text=True, creationflags=subprocess.CREATE_NO_WINDOW)
cancelled = []
owned = []
def read():
    for line in p.stderr:
        report['phases'].append(line.strip())
        if ('gemma_engine phase='+phase+' ') in line and not cancelled:
            for child in psutil.Process(p.pid).children(recursive=True):
                if pathlib.Path(child.exe()).resolve() != (root/'runtime/llama-server.exe').resolve():
                    continue
                owned.append(child)
                report['children'].append(dict(pid=child.pid, cwd=child.cwd()))
            cancelled.append(time.monotonic())
            p.stdin.close()
t = threading.Thread(target=read)
t.start()
try:
    p.wait(timeout=195)
except subprocess.TimeoutExpired:
    # Only the child broker owned by this invocation; its Job owns the model.
    report['forcedOwnedBrokerTermination'] = True
    p.kill(); p.wait(timeout=10)
exited = time.monotonic()
t.join(timeout=10)
report['exitCode'] = p.returncode
report['stdout'] = p.stdout.read()
report['cancelToExitMs'] = (exited-cancelled[0])*1000 if cancelled else None
report['childrenGone'] = bool(owned) and all(not child.is_running() for child in owned)
report['workspacesGone'] = bool(report['children']) and all(not pathlib.Path(c['cwd']).parent.exists() for c in report['children'])
report['noAdmission'] = report['stdout'] == '' and not any('phase=ready ' in line for line in report['phases'])
report['pass'] = bool(cancelled) and report['cancelToExitMs'] <= 6000 and report['childrenGone'] and report['workspacesGone'] and report['noAdmission'] and p.returncode == 1 and any(line == 'gemma_start_revoked' for line in report['phases']) and not report.get('forcedOwnedBrokerTermination')
receipt.write_text(json.dumps(report, indent=2), encoding='utf-8')
print(json.dumps(report, indent=2))
sys.exit(0 if report['pass'] else 1)
