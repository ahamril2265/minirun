# minirun learning log

One entry per step. Each entry: what we built, the kernel concepts (plain
English + an analogy), the key syscalls, commands to verify it, and what
broke.

---

## Steps 1–6 recap (built before this log existed)

These steps were done in earlier sessions; the commit messages do not record
what went wrong, so the "What broke" lines are for you to fill in from memory.

| Step | Commit | Concept |
|------|--------|---------|
| 1 | `8a510f2` | Cargo workspace (`resolver = "3"`, edition 2024) |
| 2 | `a2c2b1b` | `fork` + `execve` + `waitpid` via `std::process::Command` |
| 3 | `e940520` | clap CLI: `run [--memory] [--pids] <cmd...>` |
| 4 | `0e4b225` | UTS namespace, `sethostname` |
| 5 | `d11b734` | PID namespace, container process is PID 1 |
| 6 | `f81c905` | Mount namespace, `MS_REC|MS_PRIVATE`, fresh `/proc` |

**Key ideas to be able to explain**

- **Namespaces** wrap one global kernel resource (hostname, PID numbers, the
  mount table, ...) so a group of processes gets its own copy. Analogy: a
  hotel where each room has its own phone extension numbering.
- **`unshare(CLONE_NEWPID)` does not move the caller.** The caller keeps its
  PID; only its *next child* is born into the new namespace as PID 1. That is
  why minirun calls it in the parent before spawning, while UTS and mount
  namespaces are unshared in the child (`pre_exec`), where they apply
  immediately.
- **`pre_exec` runs between `fork` and `execve`** in the child. A forked
  copy of a multi-threaded program may have inherited a lock held by another
  thread, so only async-signal-safe work is allowed there: syscalls yes;
  `malloc`, `println!` and locks no.
- **Mount propagation.** On systemd hosts `/` is mounted `shared`, so a new
  mount namespace starts as a *peer* of the host: mounts made inside
  propagate back out. Remounting `/` with `MS_REC|MS_PRIVATE` cuts that link.
  Analogy: a copied Google Doc that is still live-linked to the original
  until you "make a copy" instead of "share".
- **`/proc` shows the PID namespace of whoever mounted it.** Without a fresh
  mount, `ps` inside the container still reads the host's `/proc`.

**Verify**
```
sudo ./target/debug/minirun run --hostname box /bin/sh -c 'hostname; echo $$; ps -o pid,cmd'
findmnt -o TARGET,PROPAGATION /        # host: expect "shared"
```

**What broke** (fill in): nested `.git` from `cargo new`; the PID namespace
catch; mounts leaking to the host through shared propagation.

---

## Part 1 Step 7: Alpine rootfs script; chroot vs pivot_root

**What we built**

`scripts/get-alpine.sh [DEST]` downloads the Alpine 3.24.4 minirootfs
(~3.7 MB), verifies it against Alpine's published SHA-256, and extracts it as
root with `--numeric-owner`. It does nothing if `DEST` already exists, because
later steps treat that directory as a read-only image layer.

**Why the extract needs root:** every file in the tarball is owned by uid 0,
and `/etc/shadow` is `0:42` (`shadow` group). A normal user cannot `chown`, so
tar would quietly make everything yours. `--numeric-owner` keeps the raw
numbers from the tarball instead of mapping names through the *host's*
`/etc/passwd`, whose ids need not match Alpine's.

**chroot vs pivot_root: the concept**

Both make a directory look like `/`. They work at different levels:

| | `chroot(dir)` | `pivot_root(new, old)` |
|---|---|---|
| What changes | The calling process's *root directory pointer* (a per-process path lookup setting) | The *root mount* of the whole mount namespace |
| Old root | Still mounted and reachable | Moved to `old`, then can be unmounted completely |
| Escape | Root can escape: `mkdir x; chroot x; cd ../../..; chroot .`. The cwd stays outside the new root, and `..` walks out | No old root left to walk back to once it is detached |
| Requirements | Any directory | `new` must be a mount point, and must not be on a shared mount |
| Used by | Build tools, recovery shells | runc, crun, every real container runtime |

Analogy: `chroot` is putting blinkers on a horse. The road is still there;
it just can't see it, and a clever horse turns its head. `pivot_root` swaps
the horse onto a different road and then demolishes the bridge back.

**Why the escape works:** the kernel only stops `..` at the process's root
when the walk passes *through* that root. `chroot` doesn't change the cwd,
so after `chroot x` the cwd is above the new root, and `..` never hits it.
That is why `chroot(2)` says it is "not intended to be used for any kind of
security purpose".

**Key syscalls (used in Step 8)**
- `mount(src, src, NULL, MS_BIND|MS_REC)`: bind the rootfs onto itself, so
  it becomes a mount point (pivot_root requirement).
- `pivot_root(".", ".")`: stack the old root *under* the new one at the same
  path, so no `put_old` directory is needed inside the image.
- `umount2(".", MNT_DETACH)`: lazily detach the old root, and the host
  filesystem is gone from the container's view.

**Verify**
```
scripts/get-alpine.sh                                  # existing dest: prints "already exists"
cat ~/containers/alpine/etc/os-release                 # Alpine v3.24
stat -c '%u:%g %n' ~/containers/alpine/etc/shadow      # 0:42
# Fresh download into a throwaway dir (needs sudo + network):
scripts/get-alpine.sh /tmp/alpine-test && sudo rm -rf /tmp/alpine-test
```

**What broke**
- The script was never committed in an earlier session and wasn't on disk;
  rewritten from scratch.
- The first draft's comment claimed the tarball has setuid files
  (`/bin/bbsuid`). `tar -tzvf | awk '$1 ~ /s/'` found none; the real reason
  for root is uid-0 ownership plus `/etc/shadow`'s gid 42. Comment fixed.
