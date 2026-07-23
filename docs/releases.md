# Releasing Goalbar

Goalbar's installed desktop app checks the latest GitHub release when it starts and every six hours
while it remains open. When it finds a newer, signature-verified version, it downloads the update,
installs it, and relaunches automatically.

The first updater-enabled build still needs to be installed manually. Every release after that can
update in place without downloading a DMG or dragging Goalbar into Applications again.

## One-time signing setup

Tauri requires every update artifact to be signed. Generate a dedicated Goalbar updater key and
keep the private key out of the repository:

```bash
pnpm tauri signer generate -w ~/.tauri/goalbar.key
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/goalbar.key
gh variable set GOALBAR_UPDATER_PUBLIC_KEY --body "$(cat ~/.tauri/goalbar.key.pub)"
```

If the key is password protected, also add its password:

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

Back up `~/.tauri/goalbar.key` and its password securely. Losing the private key prevents new
updates from being accepted by already-installed copies. Never commit or share the private key.

## Publish a release

Keep the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `package.json` aligned,
then create and push the matching tag:

```bash
git tag app-v0.2.0
git push origin app-v0.2.0
```

The Release workflow builds Apple Silicon and Intel macOS bundles, creates signed updater
artifacts, publishes the GitHub release, and generates its `latest.json` update manifest.
