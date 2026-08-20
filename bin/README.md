# `bin/chromad`

This directory is empty in git on purpose. This Mac has no Linux cross-toolchain, so committing ELF blobs from here would be fake.

Linux **x86_64** and **aarch64** helpers plus `SHA256SUMS` are built by `.github/workflows/chromad-linux.yml` and attached to GitHub Releases.

On an Omarchy machine:

```sh
./build.sh                          # preferred: compile from source
# or, after a tagged release:
./scripts/fetch-prebuilts.sh owner/repo
```

Until `bin/chromad` exists, the overlay uses `compat/chromad.sh` (grim pick-mode). Palette → theme still works.
