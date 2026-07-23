# Live test runbook

Live tests are manual and opt-in.

## Integrated browser

1. Start `pnpm tauri dev` and open Browser.
2. Load X, Reddit, and LinkedIn separately; verify every redirect remains visible in the address bar.
3. Sign in manually with dedicated non-production accounts when the website permits the embedded engine.
4. Restart and confirm expected local session persistence.
5. Resize the window and divider, switch tabs, then leave Browser; verify no native browser surface overlays another route.
6. Preview an explicit selection and visible capture before saving.
7. Confirm policy-gated collection returns manual recovery, and CAPTCHA/login/challenge screens are never bypassed.
8. Copy a draft and publish only by manually pasting and clicking the final control.
9. Type `CLEAR BROWSER DATA` in Settings and verify website sessions are removed.

## Archive import

1. Use synthetic exports first, then a redacted copy of the tester’s own official archive.
2. Check preview platform, category counts, warnings, and date range before commit.
3. Import twice and confirm the second import creates no duplicate activity items.
4. Confirm JSON export includes normalized provenance but no source path, cookies, tokens, or browser storage.

## Official API

1. Confirm `docs/platform-access.md` is current and set spend limits where available.
2. Connect through the official consent screen.
3. Publish a clearly labeled disposable test post only after confirming its exact-revision preview.
4. Test only capabilities granted to the application.
5. Revoke the token and disconnect after the run.
6. Review local logs for the secret sentinel before sharing them.

General LinkedIn DMs are not a live-test step without separately approved private access.
