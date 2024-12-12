# nrf-probe

Utility for discovering and capturing Nordic Semiconductor proprietary protocol radio packets emitted by Nordic Semiconductor nRF2401, nRF24L01+, nRF52840 and compatible transceivers.

## Description

This utility allows capturing and decoding of radio signals emitted by Nordic Semiconductor nRF2401, nRF24L01+, nRF52840 and compatible transceivers with any software defined radio (SDR) supported by [SoapySDR](https://github.com/pothosware/SoapySDR).

Currently supported features:
* Synchronization on preamble and validation of 2-byte CRC
* Supports logical addresses length between 3 and 5 bytes
* Supports payloads length between 0 and 32 bytes
* Supports dynamic payloads length using Enhanced ShockBurst
* Supports receiving on multiple channels simultaneously
* Supports 1 Mbps and 2 Mbps data rate

## Getting Started

### Dependencies

* [SoapySDR](https://github.com/pothosware/SoapySDR) is required to interact with the software defined radio. It should be available from you the packet management from your preferred distribution.
For example, for Arch Linux, assuming that a HackRF SDR is used, it can be installed using `pacman -S soapyhackrf`.
* [Rust](https://www.rust-lang.org/) as a build dependency.

### Building

To compile the binary, Rust's package manager `cargo` can be invoked as follows:

```
RUSTFLAGS="-C target-cpu=native" cargo build
```

After successful compilation, the (statically linked) binary will be available in `./target/release/nrf-probe`.

## Usage

```
USAGE:
    nrf-probe [FLAGS] [OPTIONS] --channel <channels>... --plen <payload-length>

FLAGS:
        --discover      Prints statistics about received packets instead of full packet payloads
    -h, --help          Prints help information
    -e, --shockburst    Support for Enhanced ShockBurst (ESB) packet headers, allows dynamic payload lengths
    -V, --version       Prints version information

OPTIONS:
    -a, --alen <address-length>       Address length in bytes [default: 5]  [possible values: 3, 4, 5]
    -p, --address <address-prefix>    Hexadecimal prefix of the address
    -c, --channel <channels>...       Channel selection, must be within range [1,125]
    -d, --driver <driver>             SoapySDR driver name [default: hackrf]
    -g, --gain <gain>                 Input gain in dBi [default: 20]
    -l, --plen <payload-length>       Payload length in bytes, must be within range [0,32]
    -r, --rate <rate>                 Data rate (1Mpbs or 2Mpbs) [default: 1]  [possible values: 1, 2]
    -s, --sample <sample-rate-mhz>    Sample rate in MHz
```

### Discovering 

Within the noise received by the software defined radio, there are many false positives, i.e. bit sequences starting with the correct preamble and ending with a valid CRC checksum. Filtering on additional information like payload length and logical address will significantly reduce false positives and improve performance. To discover nearby devices, their logical addresses and payload length, the parameter `--discover` can be supplied which will list the most frequently encountered logical addresses.

Example to discover nearby devices which use a 4-byte logical address and are broadcasting Enhanced ShockBurst messages:

```
$ ./nrf-probe --shockburst --alen 4 --channel 39,41,43,45,47 --discover
Address    | Count | Payload Length | Channels
1b61c5c5   |    62 | 16             | 47
194ab202   |     1 | 22             | 45
aabbd9f3   |     1 | 21             | 45
f822f2b5   |     1 | 25             | 47
156bee51   |     1 | 6              | 45
5e965159   |     1 | 23             | 43
2f287ca0   |     1 | 26             | 45
69595dba   |     1 | 4              | 43
8ae61569   |     1 | 27             | 43
e746ff72   |     1 | 17             | 45
```

Having received many packets with identical logical address (like `1b61c5c5` in above example) is a strong indicator that the signal is emitted by a real device and it is not simply background noise.

### Capturing packets emitted by nRF2401

Example to receive packets emitted by nRF2401 with fixed payload length of 25 bytes and address length of 5 bytes with prefix `0x0707` on channel 39 and channel 47 simultaneously.

```
$ ./nrf-probe --plen 25 --alen 5 --address 0707 --channel 39,41,43,45,47
 Ch Addr       Payload
 39 07070029d2 cb415d1a5ede802122a56ea4070c0842aebc7bad29df4b9519
 47 07070029d2 cb415d1a5ede802122a56ea4070c0842aebc7bad29df4b9519
 39 07070029d2 56c0aeaa430e64705d6027b175f46969499188ca361a006703
 47 07070029d2 56c0aeaa430e64705d6027b175f46969499188ca361a006703
 47 07070029d2 860e3a30e16c3c10e621dc2bb835313cf2461c50cedb02a4b8
 39 07070029d2 860e3a30e16c3c10e621dc2bb835313cf2461c50cedb02a4b8
 39 07070029d2 b610729441c0d7ebcbe7413577cd2d2bacbc3a9a36fe9b61a2
 47 07070029d2 b610729441c0d7ebcbe7413577cd2d2bacbc3a9a36fe9b61a2
```

### Capturing packets emitted by nRF24L01+

Example to receive packets Enhanced ShockBurst packets of dynamic lengths emitted by nRF24L01+ with address length of 4 bytes starting with `1b`, on channel 45, 47 and 49 simultaneously.

```
$ ./nrf-probe --shockburst --alen 4 --channel 45,47,49 --address 1b
 Ch Addr     Payload
 47 1b61c5c5 ba91fefe14d67d2bd523ec8f3d9cfd67
 47 1b61c5c5 ba91fefe14d67d2bd523ec8f3d9cfd67
 47 1b61c5c5 94ee0dc7a78e78559eb2002aa256f7b4
 47 1b61c5c5 94ee0dc7a78e78559eb2002aa256f7b4
 47 1b61c5c5 45baa548880c2bd584ae240d44d9ffdc
 47 1b61c5c5 45baa548880c2bd584ae240d44d9ffdc
 47 1b61c5c5 fe7af87fdcaac1831c80b9aa241d7900
 47 1b61c5c5 fe7af87fdcaac1831c80b9aa241d7900
 47 1b61c5c5 d7df4169576506dfdb755dbbc1871da9
 47 1b61c5c5 d7df4169576506dfdb755dbbc1871da9
```

## Authors

<a href="https://github.com/danwue/nrf-probe/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=danwue/nrf-probe" alt="contrib.rocks image" />
</a>

## Version History

* 0.0.1
    * Initial Release

## License

Distributed under the GPLv3 License. See `LICENSE.md` for more information.

## Acknowledgments

Similar and complementary projects:
* [Sniffing and decoding NRF24L01+ and Bluetooth LE packets for under $30](http://blog.cyberexplorer.me/2014/01/sniffing-and-decoding-nrf24l01-and.html)
* [Sniffing nRF24 with GNU Radio and HackRF](https://www.bitcraze.io/documentation/tutorials/hackrf-nrf/)
* [NRF24-BTLE-Decoder](https://github.com/omriiluz/NRF24-BTLE-Decoder)
* [Sniffing and Decoding NRF24L01+ and Bluetooth LE Packets with the RTL-SDR](https://www.rtl-sdr.com/sniffing-decoding-nrf24l01-bluetooth-le-packets-rtl-sdr/)
* [Sniffing “Crazyradio” NRF24 Signals with a HackRF Blue](https://www.rtl-sdr.com/sniffing-crazyradio-nrf24-signals-with-a-hackrf-blue/)
* [Decoder for NRF24L01](https://lab.dobergroup.org.ua/radiobase/portapack/portapack-eried/-/wikis/Decoder-for-NRF24L01)
* [GRCon16 - Sniffing and Dissecting nRF24L with GNU Radio and Wireshark, Marc Newlin](https://www.youtube.com/watch?v=WhsE6cwguRs)
* [RFStorm nRF24LU1+ Research Firmware](https://github.com/BastilleResearch/nrf-research-firmware)


Datasheets:
* [Datasheet: Nordic Semiconductor nRF2401](https://www.sparkfun.com/datasheets/RF/nRF2401rev1_1.pdf)
* [Datasheet: Nordic Semiconductor nRF24L01+](https://www.sparkfun.com/datasheets/Components/SMD/nRF24L01Pluss_Preliminary_Product_Specification_v1_0.pdf)
* [Datasheet: Nordic Semiconductor nRF52840](https://cdn.sparkfun.com/assets/e/c/3/1/7/Nano_BLE_MCU-nRF52840_PS_v1.1.pdf)