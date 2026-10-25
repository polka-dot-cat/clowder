# Swift signing keys for CI

`swift-signing-keys.asc` is the unmodified public-key bundle downloaded from
[Swift.org](https://www.swift.org/keys/all-keys.asc) on 2026-10-25. Its SHA-256 is
`67abcc2cff49a9dd4536b12bc4a7c2b82c434ddaa42b90516c9734f4dd5481af`.

The live endpoint intermittently returned non-PGP responses with a successful
HTTP status, preventing GPG import before the toolchain installer ran. Importing
this reviewed copy makes CI independent of that request. Swiftly still verifies
downloaded toolchain signatures; this file does not disable verification.

The active fingerprints were checked against Swift's
[installation instructions](https://www.swift.org/install/linux/tarball/):

- Automatic signing key #4: `E813C892820A6FA13755B268F167DF1ACF9CE069`
- Swift 6.x release signing key: `52BB7E3DE28A71BE22EC05FFEF80A866B47A981F`

The bundle also contains historical certificates and signed expiry extensions.
Keep the complete upstream bundle so GPG can merge those certificates.

When upgrading the pinned Swift/Swiftly versions, refresh this bundle from the
official HTTPS endpoint, review fingerprints, expiry extensions and revocations
against Swift's published signing-key information, and update this checksum.
Check it in a temporary GPG home, then verify the downloaded release signatures
and run the Linux CI jobs. An invalid download or failed signature must stop the
installation. A key rotation or revocation also requires a bundle update.
