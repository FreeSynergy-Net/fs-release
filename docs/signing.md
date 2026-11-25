# Signing with OpenPGP

Every artefact gets a detached, ASCII-armoured signature (`file.asc`), and so does `SHA256SUMS.txt`.
`.deb` (dpkg-sig) and `.rpm` (rpmsign) additionally carry their native package signature. Signing steps run
**only if the secret `GPG_PRIVATE_KEY` is set**; without it the workflow prints a warning and ships unsigned files.

The key is created and kept by the owner. Nothing in this repo generates one.

## Create the key (once, on an offline machine)

```sh
export GNUPGHOME=$(mktemp -d)
gpg --quick-generate-key "FreeSynergy Release <release@freesynergy.net>" ed25519 sign 3y
gpg --list-secret-keys --keyid-format long        # note the fingerprint
```

Ed25519 with a 3 year expiry; extending it later is a one-minute job and does not change the key.
Set a strong passphrase (the CI needs it, see below).

## Revocation certificate and backup (offline)

```sh
FPR=<fingerprint>
gpg --gen-revoke "$FPR" > revoke-FreeSynergy-release.asc     # reason: "key was compromised" or "no longer used"
gpg --armor --export-secret-keys "$FPR" > FreeSynergy-release-secret.asc
gpg --armor --export "$FPR"            > FreeSynergy-release-public.asc
```

Store the secret key and the revocation certificate on offline media (two copies, different places) and
the passphrase in the owner's password manager. Never commit any of them. If the key leaks: import the
revocation certificate, publish it, remove the secrets from GitHub, generate a new key.

## Give it to GitHub

The target state is the GitHub environment `release-signing` in EACH repository that releases, holding the secrets (`--env`; `--env-only` creates only the environment, reviewer and tag rule, the secrets follow with `--env`); the environment holds the required
reviewer, so nobody signs unseen; `docs/usage.md`, "Approval for the signing job"). Set it up with `scripts/set-signing-secrets.sh --env --dry-run` first (read it) (target organisation `FreeSynergy-Net`, overridable with `FS_ORG`; never the unrelated organisation `FreeSynergy`; the logged-in account must be admin of it and the script asks before writing),
then for real: *Settings > Environments > release-signing > Environment secrets*:

| Secret | Content |
|--------|---------|
| `GPG_PRIVATE_KEY` | full text of `FreeSynergy-release-secret.asc` |
| `GPG_PASSPHRASE` | the passphrase (empty is allowed but discouraged) |

`scripts/set-signing-secrets.sh` asks for the passphrase by typing (no echo, no file on disk; `--ask-passphrase`, default without `PASSFILE`). The passphrase must still exist as a GitHub secret `GPG_PASSPHRASE`, because the CI unlocks the signing subkey with it; locally it no longer has to lie in a file.

Organisation secrets and repository secrets are NOT protected: an environment protects only its own secrets, so a job without `environment:` in a repository that may use an organisation secret reads it
without any approval (an organisation secret is as unprotected as a repository secret, only spread over many repositories). Keep the key as environment secrets (`--env`). `scripts/set-signing-secrets.sh --env-only`
only creates the environment, the reviewer and the tag rule `v*` (for example in advance); the secrets follow with `--env`. According to the GitHub documentation (not tried here) a job with `environment:` gets the environment,
repository and organisation secrets, and with the same name the environment secret wins over the repository secret over the organisation secret.
A tag run without a key fails early (job `signing-check`) with "Secrets fehlen fuer dieses Repo/diese Organisation: GPG_PRIVATE_KEY, GPG_PASSPHRASE".
The caller workflows pass the secrets explicitly; fs-release only uses
them in the `signing-check` and `finalize` jobs, never in jobs that run build scripts of the app.
(A dedicated signing subkey that is exported without the primary key is an option for later: `gpg --export-secret-subkeys`.)

## Publish the public key

Put `FreeSynergy-release-public.asc` and its fingerprint on freesynergy.net and in the org README. Every release also contains
`FreeSynergy-release-key.pgp` (exported from the secret key in CI), but users must compare the fingerprint with the website.

## Verify as a user

```sh
gpg --import FreeSynergy-release-public.asc
gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt && sha256sum -c SHA256SUMS.txt --ignore-missing
gpg --verify tool-1.0.0-linux-x86_64.tar.gz.asc tool-1.0.0-linux-x86_64.tar.gz
dpkg-sig --verify tool_1.0.0-1_amd64.deb      # needs dpkg-sig
rpm --import FreeSynergy-release-public.asc && rpm -K tool-1.0.0-1.x86_64.rpm
gh attestation verify tool-1.0.0-linux-x86_64.tar.gz --repo FreeSynergy-Net/<repo>
```

The publish step (`actions/publish-release/publish.sh`) runs the same check before a draft becomes a Pre-release, with exactly one accepted
identity: `--signer-workflow <owner of the calling repo>/fs-release/.github/workflows/_release-finalize.yml` and `--source-ref refs/tags/<tag>`
(no second name, no fixed name). This protects against mistakes, not against someone who can change the workflow, the toolchain or the scripts.
