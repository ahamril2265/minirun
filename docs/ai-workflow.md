# How minirun was built with AI

Workflow per step: **spec (me) → AI proposal → my decision → AI draft →
review → run the tests myself → commit** (one step per commit, AI-written
code carries a `Co-Authored-By:` trailer).

Each entry records: the goal I gave, what the AI proposed and the
alternatives, what I decided and why, how it was verified, and what was wrong
and got caught.

---

## Steps 1–6

Built in earlier sessions; not recorded at the time. *(To fill in: what was
AI-drafted, and what broke — nested `.git` from `cargo new`, the PID
namespace only applying to children, mounts leaking through shared
propagation.)*

---

## Part 1 Step 7: Alpine rootfs script; chroot vs pivot_root

**Goal I gave:** verify repo state, report before changing anything, then
commit the Alpine download script and document chroot vs pivot_root.

**AI found on verification** (before any change):
- `scripts/get-alpine.sh` was missing (never committed, not on disk), though
  `~/containers/alpine` existed (Alpine 3.24.4, root-owned).
- `cargo fmt --check` failed; `.expect()` used instead of `anyhow`; no
  modules; a leftover `crates/minirun/.gitignore` from `cargo new`.

**AI proposed:**
- Rewrite the script: checksum-verified, idempotent, sudo extract.
- A separate refactor commit (Step 7.5) before Step 8, so Step 8's diff only
  shows pivot_root.
- For Step 8, `pivot_root(".", ".")` instead of the classic
  `pivot_root(new, new/.oldroot)`. The classic form must `mkdir .oldroot`
  *inside the image*, which would modify the Alpine dir that later becomes a
  read-only overlay layer.

**I decided:** approved the plan as proposed.

**Verified:**
```
$ bash -n scripts/get-alpine.sh && echo "syntax ok"
syntax ok
$ scripts/get-alpine.sh
[get-alpine] /home/ubuntu-systems/containers/alpine already exists, nothing to do
$ sha256sum -c alpine-minirootfs-3.24.4-x86_64.tar.gz.sha256     # same URLs as the script, no sudo
alpine-minirootfs-3.24.4-x86_64.tar.gz: OK
```
The full extract path needs sudo (password required, so the AI can't run it).
Not yet run end to end.

**Caught:** the AI's first draft said the tarball contains setuid files
(`/bin/bbsuid`). Listing the archive (`tar -tzvf ... | awk '$1 ~ /s/'`)
showed none. The real reason for sudo is uid-0 ownership and `/etc/shadow`
being gid 42. The comment was corrected before commit.
