Invented Cargo.lock fixtures for scripts/test_check_lock_sources.sh: `green` (one source each), `red-two-sources` (fs-types
twice), `red-v1plus` (one fs-types, but from the fs-interfaces repo: red only with the V1+ switch).
`red-old-org` (fs-types from the OLD org name FreeSynergy/fs-types) and `red-foreign-org` (fs-types from a foreign org): red with the V1+ switch, only FreeSynergy-Net/fs-types is allowed.
K-3 (registry `freesynergy`): `green-registry` (fs-types once, from `registry+https://github.com/FreeSynergy-Net/fs-crates`: green also with the V1+ switch) and `red-registry-two` (fs-types 0.0.6 and 0.0.7 from the registry, as a lock
that was not updated: red, the fix message says `cargo update -p fs-types`). `green` (fs-types from GIT FreeSynergy-Net/fs-types) is red with the V1+ switch now.
