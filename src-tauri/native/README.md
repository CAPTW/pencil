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
The explicit grant and revision are supplied by the fixture, not by a deployed
capture/revision service. Eligibility rejection is a flag-level test, not a
password-field privacy qualification. Read-only/composition branches require
their own native tests. Probe text is bounded English; Unicode/IME/undo and
timeout-after-mutation UI semantics remain to qualify.

## Next implementation work

1. Add the opt-in, target-side TSF service and authenticated, bounded local
   capture/apply bridge. Keep service lifetime, document/context identity,
   process identity, revision sink and explicit user grant bound together.
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
