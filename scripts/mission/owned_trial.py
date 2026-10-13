"""Windows-only developer test supervisor (requires installed pywin32).

One mutex and one kill-on-close Job own the entire child tree. The first child
starts suspended so no descendant can escape before Job assignment. No breakaway
is permitted. Logs contain trial output; use synthetic documents only.
"""
import argparse
import json
from pathlib import Path
import subprocess
import time

import pywintypes
import win32api
import win32con
import win32event
import win32file
import win32job
import win32process

MUTEX = r"Local\GrammarOwnedDesktopTrialV1"


def run(command, output, timeout, mutex_name=MUTEX):
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    mutex = win32event.CreateMutex(None, False, mutex_name)
    acquired = False
    job = process = thread = log = None
    result = {"status": "ERROR", "cleanup": "NOT_STARTED"}
    started = time.monotonic()
    try:
        wait = win32event.WaitForSingleObject(mutex, 0)
        if wait not in (win32con.WAIT_OBJECT_0, win32con.WAIT_ABANDONED):
            result["status"] = "SESSION_ALREADY_RUNNING"
            return result
        acquired = True
        job = win32job.CreateJobObject(None, "")
        limits = win32job.QueryInformationJobObject(job, win32job.JobObjectExtendedLimitInformation)
        limits["BasicLimitInformation"]["LimitFlags"] = win32job.JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        win32job.SetInformationJobObject(job, win32job.JobObjectExtendedLimitInformation, limits)
        sa = pywintypes.SECURITY_ATTRIBUTES()
        sa.bInheritHandle = True
        log = win32file.CreateFile(str(output / "trial.log"), win32con.GENERIC_WRITE,
                                   win32con.FILE_SHARE_READ, sa, win32con.CREATE_ALWAYS, 0, None)
        si = win32process.STARTUPINFO()
        si.dwFlags = win32con.STARTF_USESTDHANDLES
        si.hStdOutput = si.hStdError = log
        # No stdin is inherited from the interactive host.
        null = win32file.CreateFile("NUL", win32con.GENERIC_READ,
                                   win32con.FILE_SHARE_READ | win32con.FILE_SHARE_WRITE,
                                   sa, win32con.OPEN_EXISTING, 0, None)
        si.hStdInput = null
        try:
            process, thread, pid, _ = win32process.CreateProcess(
                None, subprocess.list2cmdline(command), None, None, True,
                win32con.CREATE_SUSPENDED | win32con.CREATE_NO_WINDOW, None, None, si)
        finally:
            null.Close()
        result["pid"] = pid
        try:
            win32job.AssignProcessToJobObject(job, process)
        except Exception:
            win32api.TerminateProcess(process, 1)
            win32event.WaitForSingleObject(process, 5000)
            raise
        win32process.ResumeThread(thread)
        deadline = started + timeout
        while win32event.WaitForSingleObject(process, 100) == win32con.WAIT_TIMEOUT:
            if time.monotonic() >= deadline:
                result["status"] = "TIMEOUT"
                break
        else:
            result["exitCode"] = win32process.GetExitCodeProcess(process)
            result["status"] = "PASS" if result["exitCode"] == 0 else "FAIL"
    except BaseException as error:
        result["errorType"] = type(error).__name__
    finally:
        if job is not None:
            cleanup_end = time.monotonic() + 10
            # Process signalling and Job accounting are not simultaneous.
            settle_end = time.monotonic() + 0.5
            while result.get("status") == "PASS" and time.monotonic() < settle_end:
                if not win32job.QueryInformationJobObject(job, win32job.JobObjectBasicAccountingInformation)["ActiveProcesses"]:
                    break
                time.sleep(0.01)
            waits = []
            remaining_ids = win32job.QueryInformationJobObject(job, win32job.JobObjectBasicProcessIdList)
            result["remainingProcessIds"] = list(remaining_ids)
            for child_pid in remaining_ids:
                try:
                    waits.append(win32api.OpenProcess(win32con.SYNCHRONIZE, False, child_pid))
                except pywintypes.error:
                    pass  # The Job remains the authoritative cleanup owner.
            active = win32job.QueryInformationJobObject(job, win32job.JobObjectBasicAccountingInformation)["ActiveProcesses"]
            result["remainingBeforeCleanup"] = active
            if active:
                win32job.TerminateJobObject(job, 1)
                if result["status"] == "PASS":
                    result["status"] = "FAIL_LEFTOVER_CHILDREN"
            for handle in waits:
                win32event.WaitForSingleObject(handle, max(0, int((cleanup_end - time.monotonic()) * 1000)))
                handle.Close()
            while time.monotonic() < cleanup_end:
                active = win32job.QueryInformationJobObject(job, win32job.JobObjectBasicAccountingInformation)["ActiveProcesses"]
                if active == 0:
                    break
                time.sleep(0.05)
            result["remainingAfterCleanup"] = active
            result["cleanup"] = "COMPLETE" if active == 0 else "INCOMPLETE"
            job.Close()
        for handle in (thread, process, log):
            if handle is not None:
                handle.Close()
        result["elapsedSeconds"] = round(time.monotonic() - started, 3)
        if acquired:
            (output / "supervisor-result.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
            win32event.ReleaseMutex(mutex)
        mutex.Close()
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True)
    parser.add_argument("--timeout", type=float, default=540)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or args.timeout <= 0:
        parser.error("a command and positive timeout are required")
    outcome = run(command, args.output, args.timeout)
    print(json.dumps(outcome))
    raise SystemExit(0 if outcome["status"] == "PASS" and outcome["cleanup"] == "COMPLETE" else 1)
