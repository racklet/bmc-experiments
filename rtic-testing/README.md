# rtic-testing

`rtic-testing` contains various examples and test cases for experimenting with
and learning the features of [RTIC]. These can be flashed to an
[Adafruit ItsyBitsy M4 Express] using a [CMSIS-DAP] compatible debug probe.

[RTIC]: https://rtic.rs/dev

[Adafruit ItsyBitsy M4 Express]: https://www.adafruit.com/product/3800

[CMSIS-DAP]: https://arm-software.github.io/CMSIS_5/DAP/html/index.html

## Dependencies

Two `probe-rs` based tools are needed for running/debugging. They have partially
overlapping feature sets, so it may not be necessary to install both depending
on your needs. You may `cargo install`

- `probe-run` for flashing the MCU, [RTT I/O] and stack backtraces,
- `cargo-embed` for flashing the MCU, [RTT I/O] and debugging via [GDB].

[RTT I/O]: https://github.com/probe-rs/probe-rs-rtt

[GDB]: https://www.gnu.org/software/gdb/

### Note: CMSIS-DAP support is currently broken in upstream `probe-rs`

Until [probe-rs/probe-rs#767] has made its way into a release, you may use

```shell
cargo install --git https://github.com/twelho/probe-run
cargo install --git https://github.com/twelho/cargo-embed
```

to install the two dependencies with the patch pre-applied.

[probe-rs/probe-rs#767]: https://github.com/probe-rs/probe-rs/pull/767

## Running

In this directory, with `probe-run` installed, execute

```shell
cargo run [--release] [--example <example>]
```

to compile and flash an executable to the MCU. After this `probe-run` will
automatically attach to RTT I/O so you can interact with the MCU like you would
with a standard stdin/stdout application running on your host machine. Press
`CTRL + C` to halt the MCU CPU and exit the monitoring process.

**Note:** In most cases setting up a GDB debug configuration as described below
is often not needed, since the MCU will output stack backtrace to the host's
stdout automatically on panic using [RTT I/O] thanks to the `panic_probe`
panic handler used in these binaries/examples. This feature coupled with direct
printing to the host using `rprintln!` likely provides a sufficient level of
debugging.

## Debugging

In this directory, with `cargo-embed` installed, execute

```shell
cargo embed debug [--example <example>]
```

to compile and flash an executable to the MCU. After this `cargo-embed` will
automatically start a GDB server session to which you can attach via
`localhost:3333` using a multi-arch GDB build supporting the `arm-none-eabi`
triple (e.g. `arm-none-eabi-gdb` in Arch Linux) or your favorite IDE tooling.
For example the "Embedded GDB Server" debug target in CLion has been confirmed
working when using a dummy target specifying `cargo` as the GDB server
and `embed debug [--example <example>]` as the GDB server args. Don't forget to
disable uploading and set the correct working directory under the advanced GDB
server options as well. The GDB server exposed by `cargo-embed` is still
work-in-progress, so expect bugs and missing features, but for basic breakpoints
and variable inspection it works quite well already.
