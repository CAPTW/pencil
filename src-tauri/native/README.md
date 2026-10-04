# Desktop Apply restoration (in progress)

Desktop direct Apply is a core product requirement. The Owner explicitly made
its restoration the highest priority on 2026-10-05. Copy-only is temporary
containment, not completion or permanent removal of this goal.

`tsf_apply.hpp` is an experimental target-thread component, currently exercised
only by `scripts/mission/native-apply-tsf-probe.cpp`. It is **not registered,
packaged, loaded into another app, or connected to the Grammar widget**.

The old cross-process Edit-message sequence cannot combine verification and
replacement into one operation. This component instead runs within a TSF
read/write edit session: validate single-use authority, eligibility, deadline,
revision, current source, exact UTF-16 selection offsets and composition state;
replace once; reread; report uncertainty without rollback or automatic replay.
The application/text store must obey TSF's lock contract. This does not establish
support for arbitrary desktop applications or hostile/noncompliant text stores.

The native probe runs only in Windows Sandbox on owned synthetic controls, using
an in-process TSF manager. It does not implement the cross-process integration.
The original transaction probe still supplies synthetic grants. The newer
`tsf_capture.hpp` and `native-apply-tsf-binding-probe.cpp` obtain actual source and
selection in a TSF read session and issue a single-use, context-bound grant.
The binding checks process/thread, native window, context/document identity,
visibility, enabled/read-only/password state, TOM protected/hidden formatting,
composition and an allowlist of plain input scopes before reading text. Unknown
or mixed eligibility fails closed. This is qualification of owned RichEdit50W,
not a privacy guarantee for arbitrary applications.

An actual ITfTextEditSink advances the revision after every write session,
including property-only and no-op writes. A target-thread window subclass revokes
before forwarding password, read-only, character-format, style and destruction
messages. This handles observed RichEdit reentrancy: EM_SETPASSWORDCHAR can run a
pending TSF write before updating the password style. The subclass never reads
keyboard events or text, sends input, changes focus or injects into a process.
Subscriptions are removed with the binding; sinks and grants hold weak references.

The binding is explicitly enabled by the fixture, not by a deployed opt-in UI.
Capture admission expires after five seconds; issued grants expire after thirty
seconds. These are conservative prototype bounds, not accepted product latency
or Deep budgets. Use and destruction must stay on the owning STA. Cross-process
transport, service activation, focus/navigation lifecycle, composition/physical
IME, undo and timeout-after-mutation UI semantics remain unqualified.

## Next implementation work

1. Add the opt-in, target-side TSF service and authenticated, bounded local
   capture/apply bridge. Keep service lifetime, document/context identity,
   process identity, revision sink and explicit user grant bound together. Use
   the new binding instead of manufacturing an ApplyGrant from IPC fields.
2. Obtain and recheck eligibility (including sensitive input scope) in the
   target thread before any text read; clear grants on focus/document changes,
   composition, disable, cancellation, process/bridge disconnect and expiry.
3. Connect capture and the reviewed draft to the session-bound desktop Apply
   command. A plain Win32 capture must never gain Apply authority just because
   a similarly named control exists. Keep Copy available independently.
4. Register only in a disposable guest initially; no global/system registration,
   unrelated process injection, security weakening or ordinary-profile install.
5. Qualify actual supported external editors with normal apply, R1/R2 races,
   queued edits, replay, revocation, IME, undo and post-write rereads before
   enabling Apply for them. Failure before mutation and unconfirmed mutation
   must have different outcomes. Do not reinterpret a rollback as no mutation.

## Official API rationale

- [RequestEditSession](https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfcontext-requesteditsession)
  supplies read/write edit access with distinct asynchronous completion.
- [RequestLock](https://learn.microsoft.com/en-us/windows/win32/api/textstor/nf-textstor-itextstoreacp-requestlock)
  defines the application's document-lock obligations.
- [SetText](https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfrange-settext)
  replaces a range within an edit session.
- [UIA TextPattern](https://learn.microsoft.com/en-us/dotnet/framework/ui-automation/ui-automation-textpattern-overview)
  is read-only; switching to UIA alone does not repair the verification/write race.

Build the probe from an x64 Visual Studio developer command prompt with the
installed Windows SDK (no new dependency or registration):

```cmd
cl /nologo /std:c++17 /EHsc /MT /W4 /WX /DUNICODE /D_UNICODE scripts\mission\native-apply-tsf-probe.cpp /Fe:<outside-repo>\tsf-probe.exe /Fo:<outside-repo>\tsf-probe.obj ole32.lib user32.lib uuid.lib
```

Execute in the owned Medium-integrity Sandbox: default tests RichEdit50W;
`--standard-edit` tests the standard Edit control. Nonzero exit is a failure or
unsupported environment, never an Apply pass. Guest stdout and actual editor
content rereads are the result evidence; build success alone proves neither.

For the capture/binding probe, compile `native-apply-tsf-binding-probe.cpp` with
the same flags, adding `oleaut32.lib comctl32.lib`. The final owned-Sandbox run
contains 23 scenarios (40 reported checks), including source/selection and input
scope changes reverted to their original values, queued privacy changes,
revocation, replacement captures, destroyed bindings, protected/hidden fields,
wrong-window identity, opt-out, replay and Korean/emoji source preservation.
All negative Apply cases require zero text reads and zero SetText calls; all
editor contents are reread. This is not installed-widget or external-app evidence.

Relevant API contracts:
- [OnEndEdit](https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itftexteditsink-onendedit)
  is the completion notification for a write edit session.
- [GetAppProperty](https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfcontext-getappproperty)
  reads properties maintained by the context owner; unsupported is not consent.
- [GetProtected](https://learn.microsoft.com/en-us/windows/win32/api/tom/nf-tom-itextfont-getprotected)
  distinguishes unprotected, protected and undefined formatting.

## Local channel admission (not yet connected)

`apply_channel.hpp` implements an experimental inbound transport boundary.
It creates a first-instance, single-instance local byte pipe with an explicit
logon-SID DACL and remote clients disabled. Admission pins a caller-supplied live
process handle (not a reusable PID alone), checks user/logon/session and Medium
integrity metadata, compares the kernel pipe client PID, and requires a random
256-bit capability and strictly ordered request number. Malformed requests,
replay, revocation, dead peers, timeout and broken connections invalidate admission.
The capability is removed from the returned request, never logged, and zeroed
on revocation/destruction. Request buffers are cleared before reuse and on failure.

Header/version/command/length checks precede payload allocation. UTF-16LE text
is bounded to65534 code units, rejects NUL/unpaired surrogates, and is only allowed
for the Apply message. Header and body share one absolute caller-provided deadline.
Pipe reads require nonblocking mode, so there is no abandoned blocking reader or
pending OVERLAPPED buffer. This polling reader belongs on a dedicated I/O worker,
not the editor STA. Admission itself has one owning thread; concurrent access is
not supported by this prototype.

The trusted activation/bootstrap mechanism is **not implemented**. The native
probe pins a process it creates and sends the capability through one explicitly
inherited anonymous-pipe handle, never via command line/environment/file. This
does not authenticate a deployed Grammar executable or discover a trusted broker.
An admitted Apply frame still has **no document mutation authority**: capture ID,
revision, current target, explicit action and the TSF grant must be checked by
the future target dispatcher. Bidirectional replies, STA dispatch, disconnect
revocation of queued TSF work, service activation and widget wiring remain work.
No background listener, service registration or product UI path uses this header.

Build `scripts/mission/native-apply-channel-probe.cpp` with the previous compiler
flags and `bcrypt.lib advapi32.lib`; run only in the owned Medium Sandbox.
Its28 cases use actual child processes and local pipes, including wrong peer and
capability, dead peer, replay, disable, malformed/oversized/truncated messages,
invalid UTF-16, maximum payload, duplicate server and a header/body deadline
sharing check. Different user/logon/integrity rejection, allocator faults and
hostile-process compromise are not established by these same-token fixtures.

API basis: [kernel client PID](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeclientprocessid)
and [named-pipe access control](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights).
