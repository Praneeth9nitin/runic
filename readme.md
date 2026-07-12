# runic

A container engine built from scratch in Rust. runic creates fully isolated 
Linux environments using the same kernel primitives that power Docker - 
namespaces, cgroups, overlayfs, and virtual networking.

Currently Linux-only. Mac and Windows support coming in a future release.

## What it can do

- **Hostname isolation** — each container gets its own hostname via UTS namespace
- **Filesystem isolation** — containers get their own root filesystem using overlayfs 
  and pivot_root, completely isolated from the host
- **Resource limits** — CPU, memory, and process limits enforced via cgroups v2
- **OCI image pulling** — pull any image from Docker Hub automatically, with layer 
  caching so repeated runs are instant
- **Container networking** — each container gets its own network namespace, IP 
  address, and full internet access via NAT through the host

## How to run

```bash
git clone https://github.com/Praneeth9nitin/runic
cd runic
cargo build
sudo ./target/debug/runic ubuntu:22.04 /bin/bash
```

Any Docker Hub image works:

```bash
sudo ./target/debug/runic alpine:latest /bin/sh
sudo ./target/debug/runic ubuntu:22.04 /bin/bash
```

## How it works

When you run `runic ubuntu:22.04 /bin/bash`, here's what happens:

1. **Image pulling** — runic fetches the image manifest from Docker Hub, downloads 
   each layer as a tar.gz, verifies the SHA256 hash, and extracts the layers into 
   a rootfs directory. Layers are cached locally so subsequent runs are instant.

2. **Network setup** - runic creates a bridge interface `runic0` on the host and a 
   virtual ethernet pair. One end stays on the host connected to the bridge, the 
   other end gets moved into the container.

3. **Container creation** — runic forks a child process and before it executes the 
   target program, sets up:
   - `CLONE_NEWUTS` — private hostname
   - `CLONE_NEWNS` — private mount namespace
   - `CLONE_NEWPID` — private process tree
   - `CLONE_NEWNET` — private network stack
   - overlayfs mount with the pulled image as the base layer
   - `pivot_root` to swap the container's root filesystem

4. **Networking** — the container's veth interface gets assigned IP `10.0.0.2`, 
   with the host bridge at `10.0.0.1` as the gateway. NAT rules allow the 
   container to reach the internet through the host's IP.

## Architecture
```
runic/
├── src/
│   ├── main.rs        — CLI entry point
│   ├── container.rs   — Container lifecycle
│   ├── namespace.rs   — Linux namespace setup
│   ├── filesystem.rs  — overlayfs + pivot_root
│   ├── cgroup.rs      — resource limits
│   ├── image.rs       — OCI image pulling
│   └── network.rs     — veth, bridge, NAT
```

## What's coming

- **v6** — daemon + CLI split, container state persistence
- **v7** — seccomp filters, capability dropping, rootless containers
- **v8** — full OCI runtime spec compliance

## Built with

- [nix](https://crates.io/crates/nix) — Linux syscall bindings
- [tokio](https://crates.io/crates/tokio) — async runtime
- [reqwest](https://crates.io/crates/reqwest) — HTTP client for registry API
- [anyhow](https://crates.io/crates/anyhow) — error handling