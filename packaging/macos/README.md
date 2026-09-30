# Kowalski.app

`build-app.sh` turns the `kowalski` and `kowalski-cli` binaries into `Kowalski.app` and
`Kowalski.dmg`. The Release workflow runs it on every tag with universal binaries and attaches
`Kowalski.dmg` to the release.

- `Launcher.swift` — the app's executable: starts `kowalski` from `~/.config/kowalski`, logs to
  `~/Library/Logs/Kowalski/kowalski.log`, opens the browser once the server answers, and stops
  the server when the app quits. Clicking the Dock icon opens the browser again.
- `Info.plist`, `Kowalski.icns` — bundle metadata and icon.

## Try it locally

```bash
cargo build --release -p kowalski -p kowalski-cli
packaging/macos/build-app.sh target/release/kowalski target/release/kowalski-cli 0.0.0 /tmp/kw-app
open /tmp/kw-app/Kowalski.app
```

Without signing settings the app is unsigned: fine on the machine that built it.

## Signing and notarization (release workflow)

Repository secrets, set once:

| Secret | What |
|---|---|
| `MACOS_CERT_P12` | The **Developer ID Application** certificate with its private key, exported from Keychain Access as `.p12`, then `base64 -i cert.p12 \| pbcopy` |
| `MACOS_CERT_PASSWORD` | The password chosen when exporting the `.p12` |
| `MACOS_SIGN_IDENTITY` | The certificate's name, e.g. `Developer ID Application: Jane Doe (AB12CD34EF)` (`security find-identity -v -p codesigning`) |
| `APPLE_ID` | The Apple ID email of the developer account |
| `APPLE_TEAM_ID` | The 10-character team ID (developer.apple.com → Membership) |
| `APPLE_APP_PASSWORD` | An app-specific password for that Apple ID (account.apple.com → Sign-In and Security) |

With the first three the app and the disk image are signed; with all six they are also notarized
and stapled, so Gatekeeper opens them without a warning. Missing secrets never fail the build.
