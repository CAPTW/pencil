# Public Git privacy

Use a GitHub noreply identity for this public repository. Configure each clone:

```powershell
git config --local user.name CAPTW
git config --local user.email 122129944+CAPTW@users.noreply.github.com
git config --local core.hooksPath .githooks
python scripts/check-public-email.py
```

The pre-push hook scans every outgoing ref's reachable commits, tag object and
historical blobs, including deleted files. It fails on non-public email and logs
only object IDs, never matching addresses. CI repeats the history scan. Hooks
must be enabled in each clone and can be bypassed; CI runs after publication and
cannot prevent initial disclosure. These controls reduce recurrence, not provide
an absolute guarantee. Enable GitHub account email privacy and blocking of
command-line pushes exposing personal email in account settings as well.

Reserved example domains, GitHub noreply addresses, public Claude attribution,
and the existing OpenAI URL-userinfo negative fixture are narrowly exempted.
This email gate does not replace credential scanning, payload exclusion checks,
or review of release archives and metadata. Do not commit user profiles or keys.

Correcting refs does not remove already-cloned data, cached commit pages, forks,
or artifacts. Historical package hashes and provenance remain historical; never
rewrite a receipt to pretend its original source commit was different.
