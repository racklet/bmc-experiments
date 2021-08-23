# bmc-experiments

This repository hosts experimental Rust firmware code for the BMC (and related)
microcontrollers. The research conducted here is focused on enabling Racklet BMC
functionality, such as hosting boot partition files, performing cryptographic
verification, and logging traces on ARM Cortex-M microcontrollers. We're all
still learning, and the embedded Rust ecosystem is still quite young, so feel
free to join the pioneering!

## Usage

The experiments in this repository currently target the [ATSAMD51G19A]
microcontroller (MCU) (on an [Adafruit ItsyBitsy M4 Express]) as it was readily
available, and has decent community support and enough resources for learning
and experimentation. While the final chip used for the BMC boards is still
undecided, the long term goal for the firmware and libraries here is to be
somewhat vendor and chip agnostic using abstraction layers and feature gates.
The core team doesn't have a wide variety of hardware available, but if you can
put in the effort to support and test the builds on your particular
microcontroller board, we're more than happy enable support for it!

[ATSAMD51G19A]: https://www.microchip.com/en-us/product/ATSAMD51G19A

[Adafruit ItsyBitsy M4 Express]: https://www.adafruit.com/product/3800

### Running and debugging the experiments

This repository uses [`probe-run`] to enable running (and to some extent
debugging) embedded applications just like native ones using `cargo run`. This
includes support for highly performant stdin/stdout and automatic panic message
output using the [Real-Time Transfer (RTT)] I/O protocol from the [`probe-rs`]
project. For more serious debugging a configuration for [`cargo-embed`] is
provided, which will provide a GDB server when run with `cargo embed debug
<args>` which can then be attached to using a multi-arch GDB build either from
the CLI or your favorite IDE.

[`probe-rs`]: https://probe.rs/

[`probe-run`]: https://github.com/knurling-rs/probe-run

[`cargo-embed`]: https://github.com/probe-rs/cargo-embed

[Real-Time Transfer (RTT)]: https://github.com/probe-rs/probe-rs-rtt

### Debug Probe needed

The use of `probe-rs`-based tooling (`probe-run` and `cargo-embed`) requires you
to attach to your MCU board using a [debug probe] supporting the debugging
protocol of your particular MCU (either [JTAG] or preferably [SWD] for ARM
Cortex-M based chips like the [ATSAMD51G19A]).

In addition to considering what protocol is used between the MCU and the debug
probe, it's also important to check what protocol the **debug probe uses to talk
to the host computer** (the machine where you're uploading code and debugging
from). At present (August 23rd, 2021) the `probe-rs` [supported probe protocols]
include CMSIS-DAP, ST-LINK, J-Link and experimentally FTDI (JTAG only). Feel
free to look into each to understand their differences and use what works for
you, but detailed below is the low cost and quite versatile setup that Racklet
core developers have confirmed to be working.

[debug probe]: https://developer.arm.com/tools-and-software/embedded/debug-probes

[JTAG]: https://en.wikipedia.org/wiki/JTAG

[SWD]: https://developer.arm.com/architectures/cpu-architecture/debug-visibility-and-trace/coresight-architecture/serial-wire-debug

[supported probe protocols]: https://github.com/probe-rs/probe-rs/tree/master/probe-rs/src/probe

### ST-LINK V2 clone as a CMSIS-DAP adapter

There are various low-cost [clones] of the ST-LINK V2 debugger probe available
from markets such as eBay and Aliexpress. It doesn't really matter which one you
choose/have (the original one should work as well), as the stock firmware that
only speaks the ST-LINK protocol needs to be replaced. While it could
technically work with `probe-rs`, the firmware is proprietary and has issues
talking to anything other than STM32 ARM chips. Our recommendation is flashing
the probe with the [dap42] CMSIS-DAP firmware, which enable the probe to support
basically any ARM chip (including STM32) and to work seamlessly with `probe-rs`
(well once [#767] has made its way into a release anyways, use [my `probe-rs`
fork] in the meantime). Follow the [dap42 flashing instructions] to get going,
soldering wires is most likely necessary for the initial flash, but updates can
be delivered solely via USB when flashing the combined `dapboot` image. Some
guides online may suggest soldering the SWO pin for tracing support, but this is
not necessary when using the [Real-Time Transfer (RTT)] I/O protocol.

[clones]: https://wiki.cuvoodoo.info/doku.php?id=jtag

[dap42]: https://github.com/devanlai/dap42

[#767]: https://github.com/probe-rs/probe-rs/pull/767

[my `probe-rs` fork]: https://github.com/twelho/probe-rs

[dap42 flashing instructions]: https://github.com/devanlai/dap42/blob/master/FLASHING.md

#### What about Black Magic Probe (BMP)?

[Black Magic Probe (BMP)] is an alternative, quite heavyweight debug probe
firmware that can run on *some* ST-LINK V2 clones. It provides conveniences,
such as directly hosting a GDB server from the debug probe itself. When compiled
the resulting firmware binary is roughly 100K in size, which exceeds the 64K
flash space available on the ST-LINK probes based on the STM32F103C**8** MCU.
Normally this is not an issue however, since there is a secret: all original 64K
STM32F103 chips actually have 128K of flash, it has just been disabled due to
being untested or potentially unreliable (a form of [product binning]). Now the
issue is that there are [cloned STM32 chips], and [@twelho] happens to have
a [CKSF103C8], which only has 64K physically, in his ST-LINK V2 clone. This
makes flashing the Black Magic Probe firmware impossible. Since the Racklet
project values accessibility, and we cannot guarantee that this setup will work
for everyone, we went with the [dap42] and [`probe-rs`] combination instead.
(dap42 fits into 64K comfortably, even with the `dapboot` DFU bootloader).

[Black Magic Probe (BMP)]: https://github.com/blacksphere/blackmagic

[product binning]: https://en.wikipedia.org/wiki/Product_binning

[cloned STM32 chips]: https://hackaday.com/2020/10/22/stm32-clones-the-good-the-bad-and-the-ugly/

[@twelho]: https://github.com/twelho

[CKSF103C8]: https://wtechk.com/CKS32F103.pdf

## Adafruit ItsyBitsy M4 Express Troubleshooting

### PSA: Unstable SWD debugging (in both OpenOCD and `probe-rs`)

Out of the box when connecting the ItsyBitsy to an SWD debug probe it is quite
likely that the connection will have parity errors or the probe will not be able
to detect the chip at all, regardless of the communication speed used. This has
been observed both with a [dap42] flashed ST-LINK V2 clone as well as an FTDI
FT232H based [probe from Pine64]. Additional grounding or improved power
delivery did not improve the situation in either case. [@twelho] managed to
finally trace the problem to a wrong value of the SWCLK pull-up resistor
soldered to the ItsyBitsy. Adafruit is using a 2.2 kΩ resistor instead of a 1 kΩ
one as stated by the [SAM D5x datasheet] on page 1907 to be "critical for
reliable operation". Luckily this can be fixed without needing to solder SMD
components by **wiring an additional 1.8 kΩ resistor from SWCLK to 3.3 V**,
which will result in a combined resistance of 990 Ω, close enough to the
guideline. This fix has completely eliminated all connection failures and
instability for me. Make sure to double-check your connections to avoid frying
your board and/or debug probe, remember that **you do this at your own risk**.

[probe from Pine64]: https://pine64.com/product/usb-jtag-adapter/

[SAM D5x datasheet]: https://www.microchip.com/content/dam/mchp/documents/MCU32/ProductDocuments/DataSheets/SAM_D5x_E5x_Family_Data_Sheet_DS60001507G.pdf

### Bootloader Protection enabled by default

The SAM D5x series features a bootloader protection feature that prevents
writing to the first couple of kilobytes (configurable) of flash where a
software bootloader typically resides. This feature is enabled by default to
protect the UF2 bootloader the ItsyBitsy M4 Express ships with. It needs to be
disabled via [OpenOCD] before the chip can be used with `probe-rs`. While
OpenOCD may be quite daunting for a beginner, this repository aims to provide
OpenOCD configurations that should mostly work out of the box for connecting to
the chip. (If you know of any simple OpenOCD guides, file an issue! We'd love to
have them linked here.) After fixing the SWD communication as described above
you may in an OpenOCD shell (e.g. via `telnet`) execute

- `atsame5 bootloader` to check the size of the current bootloader protection
  region in bytes,
- `atsame5 bootloader 0` to disable bootloader protection.

This process is fully reversible and only needs to be done once. Check the
details from the [OpenOCD `atsame5` documentation].

[OpenOCD]: http://openocd.org/

[OpenOCD `atsame5` documentation]: http://openocd.org/doc/html/Flash-Commands.html#atsame5

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) and
our [Code Of Conduct](CODE_OF_CONDUCT.md).

Other interesting resources include:

- [The issue tracker](https://github.com/racklet/racklet/issues)
- [The discussions forum](https://github.com/racklet/racklet/discussions)
- [The list of milestones](https://github.com/racklet/racklet/milestones)
- [The roadmap](https://github.com/orgs/racklet/projects/1)
- [The changelog](https://github.com/racklet/racklet/blob/main/CHANGELOG.md)

## Getting Help

If you have any questions about, feedback for or problems with Racklet:

- Invite yourself to the [Open Source Firmware Slack](https://slack.osfw.dev/).
- Ask a question on the [#racklet](https://osfw.slack.com/messages/racklet/)
  slack channel.
- Ask a question on
  the [discussions forum](https://github.com/racklet/racklet/discussions).
- [File an issue](https://github.com/racklet/racklet/issues/new).
- Join our [community meetings](https://hackmd.io/@racklet/Sk8jHHc7_) (see also
  the [meeting-notes](https://github.com/racklet/meeting-notes) repo).

Your feedback is always welcome!

## Maintainers

In alphabetical order:

- Dennis Marttinen, [@twelho](https://github.com/twelho)
- Jaakko Sirén, [@Jaakkonen](https://github.com/Jaakkonen)
- Lucas Käldström, [@luxas](https://github.com/luxas)
- Verneri Hirvonen, [@chiplet](https://github.com/chiplet)

## License

[Apache 2.0](LICENSE)
