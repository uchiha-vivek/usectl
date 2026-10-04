# usectl

`usectl` is a small Rust CLI for learning system performance diagnostics through
Brendan Gregg's USE methodology:

- Utilization
- Saturation
- Errors

On Linux, it reads directly from `/proc` files. On macOS, it uses native system
tools such as `top`, `uptime`, `vm_stat`, `iostat`, and `netstat`. It does not
use eBPF, perf, containers, databases, web servers, dashboards, or external
monitoring systems.

## Commands

Use from a terminal with `cargo run -- <command>`, or after `cargo install --path .`, run `usectl <command>`.

```bash
usectl cpu
usectl memory
usectl disk
usectl network
usectl analyze
usectl --json cpu
usectl --watch 2 cpu
usectl --watch 5 memory
usectl --json --watch 2 analyze
```

## Development

```bash
cargo test
cargo run -- cpu
```

This project currently supports Linux and macOS.

## License

MIT License. See `LICENSE`.

## File Map

- `Cargo.toml` defines the Rust package metadata and dependencies.
- `LICENSE` contains the MIT License terms.
- `src/main.rs` starts the application and dispatches CLI commands.
- `src/cli.rs` defines the `usectl` subcommands with `clap`.
- `src/cpu.rs` reads and parses CPU stats from Linux `/proc` or macOS `top`/`uptime`.
- `src/memory.rs` reads and parses memory, swap, and fault stats from Linux `/proc` or macOS `vm_stat`.
- `src/disk.rs` reads and parses disk counters from Linux `/proc/diskstats` or macOS `iostat`.
- `src/network.rs` reads and parses network interface counters from Linux `/proc/net/dev` or macOS `netstat`.
- `src/pressure.rs` parses Linux PSI files from `/proc/pressure/*`.
- `src/report.rs` renders the combined USE-style `analyze` report.
- `src/utils.rs` contains shared helpers for file reads and formatting.
