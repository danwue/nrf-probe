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

```
RUSTFLAGS="-C target-cpu=native" cargo build
```

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

Example to discover nearby devices which use a 5-byte logical address and are broadcasting Enhanced ShockBurst messages:

```
./nrf-probe --shockburst --alen 5 --channel 20,22,24,26,28 --discover
```

### Capturing packets emitted by nRF2401

Example to receive packets emitted by nRF2401 with fixed payload length of 25 bytes and address length of 5 bytes with prefix `0x0707` on channel 39 and channel 47 simultaneously.

```
./nrf-probe --plen 25 --alen 5 --address 0707 --channel 39,47
```

### Capturing packets emitted by nRF24L01+

Example to receive packets Enhanced ShockBurst packets of dynamic lengths emitted by nRF24L01+ with address length of 4 bytes on channel 47 and 87 simultaneously.

```
./nrf-probe --shockburst --alen 4 --channel 47,87
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