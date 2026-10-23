# Release performance measurements — 2026-10-22

Test machine: Apple M3 Pro, 18 GiB RAM, macOS 27.0.1 (26A434), Rust 1.98.1 aarch64. Measurements were collected on the real desktop while other applications were running; they are observations, not guaranteed budgets.

## Lifecycle and renderer comparison

Historical comparison before the compact-popup redesign: both renderer builds use the same egui 0.37 UI, accessibility support, 560×610 logical editor size, and generated 1600×1000 PNG. Each build contains only its selected renderer. Release settings are thin LTO and stripped binaries. `sample <pid> 1` supplies physical footprint and peak footprint; `ps` supplies RSS and CPU. These metrics describe different memory accounting and should not be substituted for one another.

| Measurement | Glow | wgpu / Metal |
| --- | ---: | ---: |
| Editor executable, measured build | 7,571,296 bytes | 11,909,488 bytes |
| Complete bundle allocated disk space (`du -sk`) | 10,272 KiB | 14,508 KiB |
| Settled editor physical footprint | 50.2M; repeat 51.3M | 42.6M |
| Observed editor peak physical footprint | 173.5M; repeat 177.5M | 245.0M |
| Editor RSS, first sample | 85,632 KiB | 108,864 KiB |
| Editor initialization to first preview UI draw, later build | 449 ms | 146 ms |

`M` above is the unit printed by Apple's `sample` tool. Initialization timings are instrumented from Rust editor initialization to the egui preview drawing callback, not to compositor presentation. The timing samples were taken at different times under desktop load; they are not a controlled latency ranking. Two glow memory launches and one wgpu memory launch were successfully sampled. The repeat wgpu window exited before another sample could be recorded.

**Decision:** retain egui and use glow by default. Glow has the smaller executable and lower observed startup peak. It did **not** have lower settled physical footprint here: wgpu was about 8M lower. Neither result supports a general claim that OpenGL uses less RAM. The editor is short-lived, and its renderer has no role in the idle resident's resource ownership. `./scripts/build-app.sh wgpu` remains available for comparison.

The separate resident measured **14.0M physical footprint**, with a 14.4M peak in the split-executable build. Startup to tray and shortcut readiness was **124–182 ms** across later release launches (138 ms in an earlier shared-executable launch). These start inside Rust and exclude LaunchServices/dynamic-loader time.

With the editor closed, `ps` confirmed that only the resident remained. A later resident sample showed **0.0% CPU**, **39,984 KiB RSS**, and cumulative CPU time of 0.44 seconds; still later readings were 33,664 KiB RSS and 0.66 seconds CPU time. RSS decreased while the desktop reclaimed shared pages. A thread sample placed the resident in AppKit's blocking event wait. These brief samples establish event-driven idle behavior, not an all-day energy profile.

The measured resident executable is 2,556,496 bytes. Its native link table contains AppKit, Foundation, Carbon, CoreGraphics, CoreVideo, ApplicationServices, and system libraries; no OpenGL or Metal renderer is linked. Screenshot pixel decoding occurs exclusively in the editor process, preventing capture allocations from becoming retained resident heap capacity.

## Storage

`cargo run --release --locked --example measure_storage` measured ten saves of a generated 1200×800 PNG with 3,841,158 bytes. Encoding the fixture was excluded. Each timed operation copied PNG bytes, wrote Markdown and JSON, synced writes, and committed metadata.

- Median: **23.59 ms**
- Minimum: **20.88 ms**
- Maximum: **29.49 ms**
- Real GUI Save + Copy of the smaller generated preview: **23 ms**, including the text clipboard operation.

The timed save retains the original PNG bytes. There is no screenshot re-encoding during save.

## Limits and reproduction

The preview fixture is generated, not a real captured desktop. It isolates editor/renderer behavior without storing sensitive screen contents. The packaged GUI was observed displaying it, receiving context, saving, and reporting the exact clipboard action. A CLI call retrieved the same snippet and context afterward.

Native selector-to-editor latency after a successful permission-enabled desktop capture remains a separate integration check. It must not be inferred from the generated fixture timings. Logs in the final build distinguish preview decoding from native capture completion and report UI draw timing without logging pixels or context.

Reproduce with:

```sh
./scripts/build-app.sh
./scripts/build-app.sh wgpu
cargo run --example benchmark_fixture
# The fixture command prints an owned temporary path; an editor removes it on close.
# Open the chosen embedded helper with --config <test-config> --capture-file <printed-path>.
# Use the same image, settings, and visible-window state for each renderer.
ps -axo pid,rss,%cpu,time,comm
sample <editor-pid> 1 -file /tmp/snapnote-editor.sample
cargo run --release --locked --example measure_storage
```

Long-duration idle CPU, repeated real captures, multiple monitors, fullscreen transitions, and mixed-DPI displays remain to be measured. No arbitrary memory or binary-size target is claimed.

## Compact popup revision

The newer 340-pixel-wide popup initialized to preview draw in 217 ms with the generated fixture. The table above describes the earlier larger editor, not a fresh memory comparison of the compact popup. The process ownership and idle resident design are unchanged.

Fresh compact-popup samples (same 1600×1000 fixture, `sample` started three seconds after process launch):

| Metric | Glow | wgpu |
| --- | ---: | ---: |
| Signed helper bytes | 7,521,664 | 11,876,224 |
| Early physical footprint | 168.7M | 238.8M |
| Peak physical footprint | 168.8M | 239.2M |
| Initialization to preview draw | 497 ms | 200 ms |
| Message finalization | 16 ms | 13 ms |

These are early samples, not settled idle measurements. Both helpers exited automatically after sampling. Early footprint remains substantial, reinforcing the separate short-lived helper design; it must not be described as a tiny resident GUI. Renderer rankings vary with measurement timing, as the earlier settled measurements demonstrate.

The final compact-popup build resident (PID 29785), with no helper running, measured **14.2M physical footprint**, **14.4M peak**, **50,480 KiB RSS**, and **0.0% CPU**. Its sampled main thread waited for native AppKit events.
