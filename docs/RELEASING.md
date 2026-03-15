# Releasing

`model-meter` publishes platform release assets through [`release.yml`](../.github/workflows/release.yml).

Current release outputs:

- Linux: `.tar.gz` archive with the CLI binary
- Windows: `.zip` archive with the CLI binary
- macOS: signed, notarized `.pkg` installer that places `model-meter` in `/usr/local/bin`

## Required GitHub Secrets For macOS Releases

The macOS release job expects these repository secrets:

- `APPLE_DEVELOPER_ID_APPLICATION_CERT_P12_BASE64`
- `APPLE_DEVELOPER_ID_APPLICATION_CERT_PASSWORD`
- `APPLE_DEVELOPER_ID_APPLICATION_IDENTITY`
- `APPLE_DEVELOPER_ID_INSTALLER_CERT_P12_BASE64`
- `APPLE_DEVELOPER_ID_INSTALLER_CERT_PASSWORD`
- `APPLE_DEVELOPER_ID_INSTALLER_IDENTITY`
- `APPLE_NOTARY_API_KEY_P8_BASE64`
- `APPLE_NOTARY_KEY_ID`
- `APPLE_NOTARY_ISSUER_ID`

What each one is:

- `APPLE_DEVELOPER_ID_APPLICATION_CERT_P12_BASE64`: base64-encoded `.p12` export of the `Developer ID Application` certificate used by `codesign`
- `APPLE_DEVELOPER_ID_APPLICATION_CERT_PASSWORD`: password used when exporting that `.p12`
- `APPLE_DEVELOPER_ID_APPLICATION_IDENTITY`: full signing identity name, for example `Developer ID Application: Your Name (TEAMID)`
- `APPLE_DEVELOPER_ID_INSTALLER_CERT_P12_BASE64`: base64-encoded `.p12` export of the `Developer ID Installer` certificate used for the installer package
- `APPLE_DEVELOPER_ID_INSTALLER_CERT_PASSWORD`: password used when exporting that `.p12`
- `APPLE_DEVELOPER_ID_INSTALLER_IDENTITY`: full installer identity name, for example `Developer ID Installer: Your Name (TEAMID)`
- `APPLE_NOTARY_API_KEY_P8_BASE64`: base64-encoded App Store Connect API key `.p8` file used by `notarytool`
- `APPLE_NOTARY_KEY_ID`: App Store Connect API key id
- `APPLE_NOTARY_ISSUER_ID`: App Store Connect issuer UUID

## Preparing The Secrets

Export the Apple certificates as `.p12` files, then base64-encode them before saving them in GitHub Actions secrets.

Examples:

```bash
base64 < developer-id-application.p12 | pbcopy
base64 < developer-id-installer.p12 | pbcopy
base64 < AuthKey_ABC1234567.p8 | pbcopy
```

Paste each encoded value into the matching GitHub secret.

## What The macOS Job Does

On macOS release builds, the workflow:

1. builds the Rust binary
2. imports the Apple signing certificates into a temporary keychain
3. signs the standalone `model-meter` binary with `Developer ID Application`
4. builds a package payload rooted at `/usr/local/bin/model-meter`
5. signs the installer package with `Developer ID Installer`
6. submits the `.pkg` to Apple notarization with `notarytool`
7. staples the notarization ticket to the `.pkg`
8. publishes the notarized `.pkg` as the macOS release asset

## Operational Notes

- Tag releases with `v*` so the package version is derived from the tag, such as `v0.1.2` -> `0.1.2`.
- Manual `workflow_dispatch` runs fall back to a numeric package version based on the GitHub run number.
- The macOS job intentionally fails early if any required signing or notarization secret is missing.

## Apple References

- [Developer ID](https://developer.apple.com/developer-id/)
- [Create Developer ID certificates](https://developer.apple.com/help/account/certificates/create-developer-id-certificates)
- [Customizing the notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)
