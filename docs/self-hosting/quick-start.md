# Quick start

This guide runs a released Linkdean image from the GitHub Container Registry with Docker Compose. You write two files, `.env` and `docker-compose.yml`, and nothing is built on your machine. A third file, `chrome-seccomp.json`, is only needed if you want page screenshots.

Published copy: [docs.linkdean.app](https://docs.linkdean.app/docs/self-hosting/quick-start) (synced from this file). Full variable reference: [environment.md](./environment.md). Operations (limits, reverse proxies, email, SSO, backups): [operations.md](./operations.md). To build from a checkout, see the README.

## License

Linkdean is [PolyForm Noncommercial License 1.0.0](https://github.com/arsamsarabi/linkdean/blob/main/LICENSE). You may self-host and fork for non-commercial purposes. Commercial use requires a separate license — contact [arsamsarabi@me.com](mailto:arsamsarabi@me.com).

## Requirements

- Docker with the Compose plugin (`docker compose version` prints v2 or later).
- A `linux/amd64` host. Images for other architectures are not published yet.
- While the GitHub repository is private, its images are private too. Sign in once with a GitHub personal access token that has the `read:packages` scope:

  ```bash
  docker login ghcr.io -u <your GitHub username>
  ```

## Images

Each release tag `vX.Y.Z` publishes:

| Tag | Points at |
| --- | --- |
| `ghcr.io/arsamsarabi/linkdean:X.Y.Z` | That release. Pin this one |
| `ghcr.io/arsamsarabi/linkdean:X.Y` | The latest patch release of `X.Y` |
| `ghcr.io/arsamsarabi/linkdean:latest` | The newest release |

The image holds the API and the web app. It listens on port `8080`, runs as UID/GID `10001`, and runs database migrations when it starts. Postgres and Meilisearch run as their own containers next to it.

## 1. Create `.env`

Make a folder for the install and generate the secrets:

```bash
mkdir linkdean && cd linkdean
printf 'POSTGRES_PASSWORD=%s\nMEILI_MASTER_KEY=%s\nSESSION_SECRET=%s\n' \
  "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" > .env
```

Then add the settings for your install. A typical homelab `.env`:

```bash
POSTGRES_PASSWORD=<generated>
MEILI_MASTER_KEY=<generated>
SESSION_SECRET=<generated>

LINKDEAN_VERSION=0.0.1
APP_URL=https://links.example.com
TRUST_FORWARDED_FOR=true
BOOTSTRAP_TOKEN=<openssl rand -hex 32>
```

Every variable is described in [environment.md](./environment.md). Keep `.env` private: it holds the database password and the search key.

## 2. Create `docker-compose.yml`

```yaml
name: linkdean

services:
  db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_USER: linkdean
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:?set POSTGRES_PASSWORD}
      POSTGRES_DB: linkdean
    volumes:
      - pgdata:/var/lib/postgresql/data
    networks: [data]
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U linkdean -d linkdean"]
      interval: 5s
      timeout: 5s
      retries: 10

  meilisearch:
    image: getmeili/meilisearch:v1.11
    restart: unless-stopped
    environment:
      MEILI_MASTER_KEY: ${MEILI_MASTER_KEY:?set MEILI_MASTER_KEY}
      MEILI_ENV: production
    volumes:
      - meili:/meili_data
    networks: [data]
    healthcheck:
      test: ["CMD", "wget", "--no-verbose", "--spider", "http://127.0.0.1:7700/health"]
      interval: 5s
      timeout: 5s
      retries: 10

  api:
    image: ghcr.io/arsamsarabi/linkdean:${LINKDEAN_VERSION:-latest}
    restart: unless-stopped
    # Passes every setting in .env to the API, so optional features need no change here.
    env_file: .env
    environment:
      DATABASE_URL: postgres://linkdean:${POSTGRES_PASSWORD:?set POSTGRES_PASSWORD}@db:5432/linkdean
      MEILI_URL: http://meilisearch:7700
    ports:
      - "8080:8080"
    volumes:
      - avatars:/app/data/avatars
      - previews:/app/data/previews
    networks: [data, browser]
    depends_on:
      db:
        condition: service_healthy
      meilisearch:
        condition: service_healthy
    healthcheck:
      test: ["CMD-SHELL", "curl -fsS http://127.0.0.1:8080/api/health >/dev/null || exit 1"]
      interval: 10s
      timeout: 5s
      retries: 12
      start_period: 40s

networks:
  data:
  # Used only by the optional chrome service, which must not reach the database.
  # Internal: no route out, so Chrome reaches the web only through the API's proxy.
  browser:
    internal: true

volumes:
  pgdata:
  meili:
  avatars:
  previews:
```

Behind a reverse proxy on the same host, publish the port on loopback only: `"127.0.0.1:8080:8080"`.

If `docker compose up` fails with `all predefined address pools have been fully subnetted`, the host has used up Docker's default network ranges (common on hosts with many stacks). Give the two networks fixed, unused subnets:

```yaml
networks:
  data:
    ipam:
      config:
        - subnet: 10.89.4.0/24
  browser:
    internal: true
    ipam:
      config:
        - subnet: 10.89.5.0/24
```

Pick ranges that `docker network inspect $(docker network ls -q) --format '{{.Name}} {{range .IPAM.Config}}{{.Subnet}}{{end}}'` does not already list.

## 3. Start it

```bash
docker compose pull
docker compose up -d
docker compose ps
```

The first start runs every migration, so `api` takes up to a minute to turn healthy. Then open `APP_URL` (or `http://<host>:8080`).

The first account becomes the instance admin. With `BOOTSTRAP_TOKEN` set, the setup page asks for it, so nobody else can claim a fresh install that is already reachable.

Check it:

```bash
curl -fsS http://localhost:8080/api/health   # process is up
curl -fsS http://localhost:8080/api/ready    # Postgres and Meilisearch reachable
```

## Optional: page screenshots with Chrome

Without Chrome, bookmark cards show the page's Open Graph image and favicon. With it, Linkdean also takes a screenshot of each saved page.

Chrome opens every page your users save, so it runs locked down: as `nobody`, with a read-only filesystem, no Linux capabilities and its own sandbox on. It sits on the `browser` network with the API and has no route to Postgres or Meilisearch.

That network is `internal`, so Chrome has no route out at all. It reaches the web only through a forward proxy inside the API, which resolves every host itself and refuses private and internal addresses. A page therefore cannot reach your LAN, the API's other networks or a cloud metadata address, even through WebSockets, service workers or DNS tricks that the API's per-request check cannot see.

### 1. Add the service

Add this under `services:` in `docker-compose.yml`:

```yaml
  chrome:
    # Pinned. Chrome renders untrusted pages, so bump this with each stable release.
    image: chromedp/headless-shell:156.0.8078.36
    restart: unless-stopped
    user: "65534:65534"
    read_only: true
    tmpfs:
      - /tmp:size=256m
    environment:
      HOME: /tmp
    cap_drop: [ALL]
    security_opt:
      - no-new-privileges:true
      - seccomp=./chrome-seccomp.json
    shm_size: 256mb
    mem_limit: 1g
    pids_limit: 512
    networks: [browser]
    # Runs Chrome directly, because the image's entrypoint would turn the sandbox off.
    # Chrome binds DevTools to localhost only; socat exposes it on port 9222.
    entrypoint: ["sh", "-c"]
    command:
      - >-
        socat TCP4-LISTEN:9222,fork TCP4:127.0.0.1:9223 &
        exec /headless-shell/headless-shell
        --use-gl=angle --use-angle=swiftshader
        --remote-debugging-address=127.0.0.1 --remote-debugging-port=9223
        --hide-scrollbars --font-render-hinting=none
        --proxy-server=http://api:3128 '--proxy-bypass-list=<-loopback>'
        --force-webrtc-ip-handling-policy=disable_non_proxied_udp
```

### 2. Point the API at it

Add to `.env`:

```bash
BROWSER_WS_URL=http://chrome:9222
BROWSER_PROXY_BIND=0.0.0.0:3128
```

`BROWSER_PROXY_BIND` starts the proxy Chrome's `--proxy-server` flag points at. Without it Chrome, on its internal network, cannot load any page.

### 3. Create `chrome-seccomp.json`

Chrome's sandbox needs user namespaces, which Docker's default seccomp profile blocks. This profile is Docker's default plus `clone`, `unshare`, `setns` and `chroot`. Save it as `chrome-seccomp.json` next to `docker-compose.yml`.

<details>
<summary>chrome-seccomp.json</summary>

<!-- seccomp:start -->
```json
{
  "defaultAction": "SCMP_ACT_ERRNO",
  "defaultErrnoRet": 1,
  "archMap": [{"architecture": "SCMP_ARCH_X86_64", "subArchitectures": ["SCMP_ARCH_X86", "SCMP_ARCH_X32"]}, {"architecture": "SCMP_ARCH_AARCH64", "subArchitectures": ["SCMP_ARCH_ARM"]}, {"architecture": "SCMP_ARCH_MIPS64", "subArchitectures": ["SCMP_ARCH_MIPS", "SCMP_ARCH_MIPS64N32"]}, {"architecture": "SCMP_ARCH_MIPS64N32", "subArchitectures": ["SCMP_ARCH_MIPS", "SCMP_ARCH_MIPS64"]}, {"architecture": "SCMP_ARCH_MIPSEL64", "subArchitectures": ["SCMP_ARCH_MIPSEL", "SCMP_ARCH_MIPSEL64N32"]}, {"architecture": "SCMP_ARCH_MIPSEL64N32", "subArchitectures": ["SCMP_ARCH_MIPSEL", "SCMP_ARCH_MIPSEL64"]}, {"architecture": "SCMP_ARCH_S390X", "subArchitectures": ["SCMP_ARCH_S390"]}, {"architecture": "SCMP_ARCH_RISCV64", "subArchitectures": null}, {"architecture": "SCMP_ARCH_LOONGARCH64", "subArchitectures": null}],
  "syscalls": [
    {"names": ["accept", "accept4", "access", "adjtimex", "alarm", "bind", "brk", "cachestat", "capget", "capset", "chdir", "chmod", "chown", "chown32", "clock_adjtime", "clock_adjtime64", "clock_getres", "clock_getres_time64", "clock_gettime", "clock_gettime64", "clock_nanosleep", "clock_nanosleep_time64", "close", "close_range", "connect", "copy_file_range", "creat", "dup", "dup2", "dup3", "epoll_create", "epoll_create1", "epoll_ctl", "epoll_ctl_old", "epoll_pwait", "epoll_pwait2", "epoll_wait", "epoll_wait_old", "eventfd", "eventfd2", "execve", "execveat", "exit", "exit_group", "faccessat", "faccessat2", "fadvise64", "fadvise64_64", "fallocate", "fanotify_mark", "fchdir", "fchmod", "fchmodat", "fchmodat2", "fchown", "fchown32", "fchownat", "fcntl", "fcntl64", "fdatasync", "fgetxattr", "flistxattr", "flock", "fork", "fremovexattr", "fsetxattr", "fstat", "fstat64", "fstatat64", "fstatfs", "fstatfs64", "fsync", "ftruncate", "ftruncate64", "futex", "futex_requeue", "futex_time64", "futex_wait", "futex_waitv", "futex_wake", "futimesat", "getcpu", "getcwd", "getdents", "getdents64", "getegid", "getegid32", "geteuid", "geteuid32", "getgid", "getgid32", "getgroups", "getgroups32", "getitimer", "getpeername", "getpgid", "getpgrp", "getpid", "getppid", "getpriority", "getrandom", "getresgid", "getresgid32", "getresuid", "getresuid32", "getrlimit", "get_robust_list", "getrusage", "getsid", "getsockname", "getsockopt", "get_thread_area", "gettid", "gettimeofday", "getuid", "getuid32", "getxattr", "getxattrat", "inotify_add_watch", "inotify_init", "inotify_init1", "inotify_rm_watch", "io_cancel", "ioctl", "io_destroy", "io_getevents", "io_pgetevents", "io_pgetevents_time64", "ioprio_get", "ioprio_set", "io_setup", "io_submit", "ipc", "kill", "landlock_add_rule", "landlock_create_ruleset", "landlock_restrict_self", "lchown", "lchown32", "lgetxattr", "link", "linkat", "listen", "listmount", "listxattr", "listxattrat", "llistxattr", "_llseek", "lremovexattr", "lseek", "lsetxattr", "lstat", "lstat64", "madvise", "map_shadow_stack", "membarrier", "memfd_create", "memfd_secret", "mincore", "mkdir", "mkdirat", "mknod", "mknodat", "mlock", "mlock2", "mlockall", "mmap", "mmap2", "mprotect", "mq_getsetattr", "mq_notify", "mq_open", "mq_timedreceive", "mq_timedreceive_time64", "mq_timedsend", "mq_timedsend_time64", "mq_unlink", "mremap", "mseal", "msgctl", "msgget", "msgrcv", "msgsnd", "msync", "munlock", "munlockall", "munmap", "name_to_handle_at", "nanosleep", "newfstatat", "_newselect", "open", "openat", "openat2", "pause", "pidfd_open", "pidfd_send_signal", "pipe", "pipe2", "pkey_alloc", "pkey_free", "pkey_mprotect", "poll", "ppoll", "ppoll_time64", "prctl", "pread64", "preadv", "preadv2", "prlimit64", "process_mrelease", "pselect6", "pselect6_time64", "pwrite64", "pwritev", "pwritev2", "read", "readahead", "readlink", "readlinkat", "readv", "recv", "recvfrom", "recvmmsg", "recvmmsg_time64", "recvmsg", "remap_file_pages", "removexattr", "removexattrat", "rename", "renameat", "renameat2", "restart_syscall", "riscv_hwprobe", "rmdir", "rseq", "rt_sigaction", "rt_sigpending", "rt_sigprocmask", "rt_sigqueueinfo", "rt_sigreturn", "rt_sigsuspend", "rt_sigtimedwait", "rt_sigtimedwait_time64", "rt_tgsigqueueinfo", "sched_getaffinity", "sched_getattr", "sched_getparam", "sched_get_priority_max", "sched_get_priority_min", "sched_getscheduler", "sched_rr_get_interval", "sched_rr_get_interval_time64", "sched_setaffinity", "sched_setattr", "sched_setparam", "sched_setscheduler", "sched_yield", "seccomp", "select", "semctl", "semget", "semop", "semtimedop", "semtimedop_time64", "send", "sendfile", "sendfile64", "sendmmsg", "sendmsg", "sendto", "setfsgid", "setfsgid32", "setfsuid", "setfsuid32", "setgid", "setgid32", "setgroups", "setgroups32", "setitimer", "setpgid", "setpriority", "setregid", "setregid32", "setresgid", "setresgid32", "setresuid", "setresuid32", "setreuid", "setreuid32", "setrlimit", "set_robust_list", "setsid", "setsockopt", "set_thread_area", "set_tid_address", "setuid", "setuid32", "setxattr", "setxattrat", "shmat", "shmctl", "shmdt", "shmget", "shutdown", "sigaltstack", "signalfd", "signalfd4", "sigprocmask", "sigreturn", "socketcall", "socketpair", "splice", "stat", "stat64", "statfs", "statfs64", "statmount", "statx", "symlink", "symlinkat", "sync", "sync_file_range", "syncfs", "sysinfo", "tee", "tgkill", "time", "timer_create", "timer_delete", "timer_getoverrun", "timer_gettime", "timer_gettime64", "timer_settime", "timer_settime64", "timerfd_create", "timerfd_gettime", "timerfd_gettime64", "timerfd_settime", "timerfd_settime64", "times", "tkill", "truncate", "truncate64", "ugetrlimit", "umask", "uname", "unlink", "unlinkat", "uretprobe", "utime", "utimensat", "utimensat_time64", "utimes", "vfork", "vmsplice", "wait4", "waitid", "waitpid", "write", "writev"], "action": "SCMP_ACT_ALLOW"},
    {"names": ["process_vm_readv", "process_vm_writev", "ptrace"], "action": "SCMP_ACT_ALLOW", "includes": {"minKernel": "4.8"}},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 3, "op": "SCMP_CMP_LT"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 7, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 8, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 9, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 10, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 13, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 14, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 15, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 16, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 17, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 18, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 21, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 22, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 24, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 26, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 27, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 28, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 29, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 30, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 31, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 32, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 33, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 35, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 36, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 39, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 41, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 42, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 43, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 44, "op": "SCMP_CMP_EQ"}]},
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 45, "op": "SCMP_CMP_EQ"}]},
    {"names": ["personality"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 0, "op": "SCMP_CMP_EQ"}]},
    {"names": ["personality"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 8, "op": "SCMP_CMP_EQ"}]},
    {"names": ["personality"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 131072, "op": "SCMP_CMP_EQ"}]},
    {"names": ["personality"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 131080, "op": "SCMP_CMP_EQ"}]},
    {"names": ["personality"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 4294967295, "op": "SCMP_CMP_EQ"}]},
    {"names": ["sync_file_range2", "swapcontext"], "action": "SCMP_ACT_ALLOW", "includes": {"arches": ["ppc64le"]}},
    {"names": ["arm_fadvise64_64", "arm_sync_file_range", "sync_file_range2", "breakpoint", "cacheflush", "set_tls"], "action": "SCMP_ACT_ALLOW", "includes": {"arches": ["arm", "arm64"]}},
    {"names": ["arch_prctl"], "action": "SCMP_ACT_ALLOW", "includes": {"arches": ["amd64", "x32"]}},
    {"names": ["modify_ldt"], "action": "SCMP_ACT_ALLOW", "includes": {"arches": ["amd64", "x32", "x86"]}},
    {"names": ["s390_pci_mmio_read", "s390_pci_mmio_write", "s390_runtime_instr"], "action": "SCMP_ACT_ALLOW", "includes": {"arches": ["s390", "s390x"]}},
    {"names": ["riscv_flush_icache"], "action": "SCMP_ACT_ALLOW", "includes": {"arches": ["riscv64"]}},
    {"names": ["open_by_handle_at"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_DAC_READ_SEARCH"]}},
    {"names": ["bpf", "clone", "clone3", "fanotify_init", "fsconfig", "fsmount", "fsopen", "fspick", "lookup_dcookie", "lsm_get_self_attr", "lsm_list_modules", "lsm_set_self_attr", "mount", "mount_setattr", "move_mount", "open_tree", "perf_event_open", "quotactl", "quotactl_fd", "setdomainname", "sethostname", "setns", "syslog", "umount", "umount2", "unshare"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_SYS_ADMIN"]}},
    {"names": ["clone"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 0, "value": 2114060288, "op": "SCMP_CMP_MASKED_EQ"}], "excludes": {"caps": ["CAP_SYS_ADMIN"], "arches": ["s390", "s390x"]}},
    {"names": ["clone"], "action": "SCMP_ACT_ALLOW", "args": [{"index": 1, "value": 2114060288, "op": "SCMP_CMP_MASKED_EQ"}], "comment": "s390 parameter ordering for clone is different", "includes": {"arches": ["s390", "s390x"]}, "excludes": {"caps": ["CAP_SYS_ADMIN"]}},
    {"names": ["clone3"], "action": "SCMP_ACT_ERRNO", "errnoRet": 38, "excludes": {"caps": ["CAP_SYS_ADMIN"]}},
    {"names": ["reboot"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_SYS_BOOT"]}},
    {"names": ["chroot"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_SYS_CHROOT"]}},
    {"names": ["delete_module", "init_module", "finit_module"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_SYS_MODULE"]}},
    {"names": ["acct"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_SYS_PACCT"]}},
    {"names": ["kcmp", "pidfd_getfd", "process_madvise", "process_vm_readv", "process_vm_writev", "ptrace"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_SYS_PTRACE"]}},
    {"names": ["iopl", "ioperm"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_SYS_RAWIO"]}},
    {"names": ["settimeofday", "stime", "clock_settime", "clock_settime64"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_SYS_TIME"]}},
    {"names": ["vhangup"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_SYS_TTY_CONFIG"]}},
    {"names": ["get_mempolicy", "mbind", "set_mempolicy", "set_mempolicy_home_node"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_SYS_NICE"]}},
    {"names": ["syslog"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_SYSLOG"]}},
    {"names": ["bpf"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_BPF"]}},
    {"names": ["perf_event_open"], "action": "SCMP_ACT_ALLOW", "includes": {"caps": ["CAP_PERFMON"]}},
    {"names": ["clone", "unshare", "setns", "chroot"], "action": "SCMP_ACT_ALLOW", "comment": "Linkdean: Chrome's namespace sandbox creates user, PID and network namespaces and chroots its renderers. Everything else is Docker's default profile (moby/profiles@4b8214fc4971e06172938a1f1554ccff6647ee80)."}
  ]
}
```
<!-- seccomp:end -->

</details>

### 4. Restart

```bash
docker compose up -d
```

### Hosts that restrict user namespaces (Ubuntu 23.10 and later)

Ubuntu 23.10 and later, including 24.04 LTS, block unprivileged user namespaces through AppArmor. Chrome's sandbox needs them, so Chrome exits with `No usable sandbox!` in `docker compose logs chrome`, and cards fall back to Open Graph images and favicons.

Check the host:

```bash
sysctl kernel.apparmor_restrict_unprivileged_userns
```

`1` means restricted. To allow them, and keep the setting across reboots:

```bash
echo 'kernel.apparmor_restrict_unprivileged_userns=0' | sudo tee /etc/sysctl.d/60-userns-chrome.conf
sudo sysctl --system | grep userns
docker compose up -d chrome
```

The last `sysctl` line should read `kernel.apparmor_restrict_unprivileged_userns = 0`. Chrome's log should then show `DevTools listening on ...` and no `No usable sandbox!`.

This is a host-wide trade-off: it lets every unprivileged process on the host create user namespaces again, not just Chrome. Ubuntu restricts them because they widen the kernel attack surface. If you don't want that on the host, skip screenshots instead.

To undo it:

```bash
sudo rm /etc/sysctl.d/60-userns-chrome.conf
sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=1
```

Then remove the `chrome` service and `BROWSER_WS_URL`, or leave Chrome running without screenshots.
