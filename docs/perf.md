# Performance Budgets & Profiles — MCLRE

> **Source:** SPEC.md Section 9 (Performance is a feature). Budgets are mandatory gates for M3, M14, and v1.0.
> **Hardware profiles:** Defined here so CI and local runs share identical targets.

---

## 1. Hardware Profiles (Reference Machines)

| Profile | CPU | GPU | RAM | OS | Notes |
|---------|-----|-----|-----|-----|-------|
| **Low** | Intel i3-6100 / Ryzen 3 1200 (2C/4T, ~3.5 GHz) | Intel HD 530 / Vega 3 (integrated) | 8 GB DDR4-2133 | Win10 21H2 / Ubuntu 22.04 | Integrated graphics, no discrete GPU |
| **Mid** | Intel i5-10400 / Ryzen 5 3600 (6C/12T, ~4.0 GHz) | GTX 1660 Super / RX 5600 XT (6 GB VRAM) | 16 GB DDR4-3200 | Win11 23H2 / Ubuntu 24.04 | Typical 2020-2022 gaming laptop/desktop |
| **High** | Intel i7-13700K / Ryzen 9 7900X | RTX 4070 / RX 7900 XT (12+ GB VRAM) | 32 GB DDR5-5600 | Win11 24H2 / Ubuntu 24.04 | Enthusiast desktop, future-proofing |

> **CI runs on GitHub `windows-latest` (≈ Mid profile) and `ubuntu-latest` (≈ Mid profile).**
> Local development should test at least one profile before pushing.

---

## 2. Budget Definitions (Frame-Time = 16.67 ms @ 60 FPS)

### 2.1 Render Budget (per frame)

| Stage | Low Budget | Mid Budget | High Budget | Measurement |
|-------|------------|------------|-------------|-------------|
| **Shadow Pass** | 1.5 ms | 1.0 ms | 0.5 ms | GPU timer query |
| **G-Buffer (Opaque)** | 3.0 ms | 2.0 ms | 1.0 ms | GPU timer query |
| **G-Buffer (Cutout)** | 1.5 ms | 1.0 ms | 0.5 ms | GPU timer query |
| **Sky / Clouds** | 1.0 ms | 0.5 ms | 0.2 ms | GPU timer query |
| **Translucent** | 2.0 ms | 1.0 ms | 0.5 ms | GPU timer query |
| **Composite / Post** | 1.5 ms | 1.0 ms | 0.5 ms | GPU timer query |
| **UI** | 1.0 ms | 0.5 ms | 0.2 ms | CPU + GPU |
| **Total GPU** | **≤ 11.5 ms** | **≤ 7.0 ms** | **≤ 3.4 ms** | Sum of stages |
| **CPU Frame Prep** | 3.0 ms | 2.0 ms | 1.0 ms | `std::time::Instant` |
| **Total Frame** | **≤ 16.67 ms** | **≤ 16.67 ms** | **≤ 16.67 ms** | End-to-end |

> **Hard limit:** If *any* profile exceeds 16.67 ms total frame time on its target hardware, the build fails CI.

### 2.2 Server Tick Budget (20 TPS = 50 ms per tick)

| System | Low Budget | Mid Budget | High Budget |
|--------|------------|------------|-------------|
| Packet Processing | 2.0 ms | 1.5 ms | 1.0 ms |
| Player Tick (×N) | 5.0 ms | 3.0 ms | 1.5 ms |
| Block Actions | 3.0 ms | 2.0 ms | 1.0 ms |
| Chunk Loading/Streaming | 8.0 ms | 5.0 ms | 2.0 ms |
| Entity AI / Physics | 10.0 ms | 6.0 ms | 3.0 ms |
| Redstone / Fluids | 5.0 ms | 3.0 ms | 1.5 ms |
| Autosave Scheduling | 1.0 ms | 0.5 ms | 0.2 ms |
| Outbound Flush | 2.0 ms | 1.0 ms | 0.5 ms |
| **Total Tick** | **≤ 36 ms** | **≤ 22 ms** | **≤ 10.7 ms** |

> **Headroom:** 50 ms budget − total = margin for "can't keep up" logging and GC.

### 2.3 Memory Budgets

| Component | Low | Mid | High |
|-----------|-----|-----|------|
| **Client Total (RSS)** | ≤ 1.5 GB | ≤ 2.5 GB | ≤ 4 GB |
| - Texture Atlas (VRAM) | ≤ 256 MB | ≤ 512 MB | ≤ 1 GB |
| - Chunk Cache (RAM) | ≤ 512 MB | ≤ 1 GB | ≤ 2 GB |
| - UI / Assets | ≤ 128 MB | ≤ 256 MB | ≤ 512 MB |
| **Server Total (RSS)** | ≤ 1 GB | ≤ 2 GB | ≤ 4 GB |
| - Region Cache | ≤ 256 MB | ≤ 512 MB | ≤ 1 GB |
| - Entity / Chunk Data | ≤ 512 MB | ≤ 1 GB | ≤ 2 GB |

> **Infinite world stress test (M13):** Must stay ≤ 2 GB client RSS after 10k blocks exploration.

### 2.4 Startup & Load Times

| Operation | Low | Mid | High |
|-----------|-----|-----|------|
| **Cold Boot → Main Menu** | ≤ 8 s | ≤ 4 s | ≤ 2 s |
| **World Create (Medium)** | ≤ 5 s | ≤ 3 s | ≤ 1.5 s |
| **World Load (Existing)** | ≤ 6 s | ≤ 3 s | ≤ 1.5 s |
| **Dimension Switch (Nether)** | ≤ 4 s | ≤ 2 s | ≤ 1 s |
| **Asset Pack Load (Vanilla)** | ≤ 3 s | ≤ 1.5 s | ≤ 0.5 s |

---

## 3. Profiling Tooling

### 3.1 CPU Profiling
```bash
# Flamegraph (Linux/macOS)
cargo install flamegraph
cargo flamegraph --bin mclre -- --width 1280 --height 720

# Windows: use `tracy` or `superluminal`
# cargo install tracy-client
# TRACY=1 cargo run --release --bin mclre
```

### 3.2 GPU Profiling
- **Windows:** PIX, RenderDoc, NVIDIA Nsight Graphics, AMD Radeon GPU Profiler
- **Linux:** RenderDoc, NVIDIA Nsight, AMD Radeon GPU Profiler
- **In-app:** `RUST_LOG=debug` + `wgpu` trace layer (`WGPU_TRACE=1`)

### 3.3 Automated Benchmarks (CI)
```toml
# benches/frame_time.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mclre_client::{Client, ClientConfig};

fn bench_frame_time(c: &mut Criterion) {
    let mut client = Client::new(ClientConfig::default()).unwrap();
    c.bench_function("frame_time_100_frames", |b| {
        b.iter(|| {
            for _ in 0..100 {
                client.render_frame(black_box(0.016));
            }
        });
    });
}
criterion_group!(benches, bench_frame_time);
criterion_main!(benches);
```

---

## 4. Regression Detection (CI Gates)

| Check | Threshold | Fail Build? |
|-------|-----------|-------------|
| `cargo bench` frame time (Mid) | > 16.67 ms avg | ✅ Yes |
| `cargo bench` server tick (Mid) | > 40 ms avg | ✅ Yes |
| Client RSS (Mid, 5 min play) | > 2.5 GB | ✅ Yes |
| Server RSS (Mid, 10 players) | > 2 GB | ✅ Yes |
| Startup time (Mid) | > 4 s | ✅ Yes |
| World load (Mid, Medium) | > 3 s | ✅ Yes |

> **CI runs on `windows-latest` and `ubuntu-latest` (both ≈ Mid profile).**
> Results uploaded as artifacts for trend analysis.

---

## 5. Optimization Checklist (Per Milestone)

| Milestone | Required Optimizations |
|-----------|------------------------|
| **M3** | Frustum culling, upload budget, priority queue, basic occlusion |
| **M4** | Packet batching, chunk streaming priority, connection coalescing |
| **M5** | Spatial partitioning for entities, redstone update throttling |
| **M6** | Parallel chunk gen, biome cache, structure bounding boxes |
| **M7** | AI goal batching, pathfinding cache, mob cap enforcement |
| **M8** | UI layout caching, texture atlas defrag, font atlas sharing |
| **M9** | UDP packet compression, delta chunk encoding, prediction |
| **M10** | Split-screen view culling, shared resource pools |
| **M11** | Audio occlusion culling, stream pooling, HRTF fallback |
| **M12** | WASM fuel limiting, host function caching, mod isolation |
| **M13** | Region LRU, async flush, backup pruning |
| **M14** | **Final polish: profile-guided optimization (PGO), LTO, strip** |

---

## 6. Hardware-Specific Notes

### Windows
- Prefer **Vulkan** backend (lowest overhead), fallback DX12 → GL
- Enable **D3D12 debug layer** in dev: `DX_DEBUG=1`
- Test on **Hybrid GPU** laptops (force discrete GPU via `D3D12_DEFAULT_ADAPTER`)

### Linux
- **Vulkan** primary, **GL** fallback
- Wayland: test with `winit` Wayland backend
- Mesa: test `RADV_PERFTEST=aco` (AMD), `MESA_VK_DEVICE_SELECT` (Intel)

### macOS (Future)
- **Metal** only via `wgpu`
- Test on Apple Silicon (M1/M2/M3) + Intel

---

## 7. Budget Validation Commands

```bash
# Quick local validation (Mid profile simulation)
cargo run --release --bin mclre -- --width 1280 --height 720 --benchmark-frames 600

# Server tick benchmark
cargo run --release --bin mclre-server -- --benchmark-ticks 1000 --players 10

# Memory stress (client)
cargo run --release --bin worldgen-stress -- --radius 313 --no-memory-check

# Full test suite
cargo test --workspace --release
cargo bench --workspace
```

---

## 8. Owner Sign-Off

| Profile | Validated By | Date | Notes |
|---------|--------------|------|-------|
| Low | — | — | Pending hardware access |
| Mid | — | — | CI baseline |
| High | — | — | Optional |

> **Update this table** when profiles are validated on physical hardware.