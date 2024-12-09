use bit_vec::BitVec;
use crc::Algorithm;
use rustradio::block::{Block, BlockEOF, BlockName, BlockRet};
use rustradio::stream::{NoCopyStream, NoCopyStreamp, Streamp};
use rustradio::Error;
use std::fmt::{Display, Formatter};

const CRC16: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_3740);
const CRC16_SB: crc::Crc<u16> = crc::Crc::<u16>::new(&Algorithm {
    init: 0x3c18, // custom init to account for left padding due to 9-bit PCF field.
    ..crc::CRC_16_IBM_3740
});

#[derive(Debug, Clone)]
enum NrfDecoder {
    Empty,
    // synchronizing on preamble
    Sync(usize, bool),
    // matching and receiving address
    RecvAddr(BitVec, BitVec),
    // matching and receiving PCF header
    RecvHeader(BitVec, BitVec),
    // receiving payload
    RecvPayload(usize, BitVec),
    // matching CRC header
    CheckCrc(BitVec, BitVec),
}

#[derive(Debug, Clone)]
pub struct NrfConfig {
    // channel (0-125)
    channel: u8,
    // address length (3-5)
    address_len: usize,
    // address prefix, in reverse order from LSB to MSB
    address_prefix: BitVec,
    payload_len: Option<usize>,
    shockburst: bool,
}

impl NrfConfig {
    pub fn fixed_length(
        channel: u8,
        address_len: usize,
        payload_len: usize,
        address_prefix: &[u8],
    ) -> Self {
        let revered_address_prefix: Vec<u8> = address_prefix
            .iter()
            .rev()
            .map(|x| x.reverse_bits())
            .collect();
        NrfConfig {
            channel,
            address_len,
            address_prefix: BitVec::from_bytes(&revered_address_prefix),
            payload_len: Some(payload_len),
            shockburst: false,
        }
    }
    pub fn shockburst(
        channel: u8,
        address_len: usize,
        payload_len: Option<usize>,
        address_prefix: &[u8],
    ) -> Self {
        let revered_address_prefix: Vec<u8> = address_prefix
            .iter()
            .rev()
            .map(|x| x.reverse_bits())
            .collect();
        NrfConfig {
            channel,
            address_len,
            address_prefix: BitVec::from_bytes(&revered_address_prefix),
            payload_len,
            shockburst: true,
        }
    }

    // maximum possible length of message in bytes used for buffer initialization to avoid reallocations
    fn max_length_bytes(&self) -> usize {
        let crc_len = 2;
        let payload_len = self.payload_len.unwrap_or(32);
        let header_len = if self.shockburst { 2 } else { 0 };
        self.address_len + header_len + payload_len + crc_len
    }

    // prepended length of padding in bits required for messages which are not byte-aligned
    // this is used for ShockBurst messages which have a 9-bit PCF header, otherwise set to 0
    fn padding_length_bits(&self) -> usize {
        if self.shockburst {
            7
        } else {
            0
        }
    }

    // length of header in bits if present, otherwise set to 0
    fn header_length_bits(&self) -> usize {
        if self.shockburst {
            9
        } else {
            0
        }
    }

    // length within PCF in reverse order from LSB to MSB
    // for messages without ShockBurst PCF header or if length is unknown, empty collection is returned
    fn length_header_bits_rev(&self) -> BitVec {
        self.payload_len.map_or(BitVec::with_capacity(0), |len| {
            let mut b = BitVec::from_bytes(&[(len as u8).reverse_bits()]);
            b.truncate(6);
            b
        })
    }
}

impl NrfDecoder {
    // called when parsing failed, drops the first bit of the received data and re-attempts parsing
    fn drop_bit_and_resync(
        config: &NrfConfig,
        out: &NoCopyStreamp<NrfFrame>,
        data: BitVec,
    ) -> NrfDecoder {
        data.iter()
            .skip(config.padding_length_bits())
            .fold(NrfDecoder::Sync(7, !data[0]), |d, b| {
                d.push_bit(config, out, b)
            })
    }

    fn push_bit(self, config: &NrfConfig, out: &NoCopyStreamp<NrfFrame>, bit: bool) -> NrfDecoder {
        match self {
            NrfDecoder::Empty => NrfDecoder::Sync(1, bit),

            // receiving preamble
            NrfDecoder::Sync(recv, prev) if prev != bit && recv < 8 => {
                NrfDecoder::Sync(recv + 1, bit)
            }

            // received preamble, check first bit of address
            NrfDecoder::Sync(8, prev) if prev != bit => {
                let mut buffer = BitVec::with_capacity(config.max_length_bytes() * 8);
                buffer.grow(config.padding_length_bits(), false);
                NrfDecoder::RecvAddr(buffer, config.address_prefix.clone())
                    .push_bit(config, out, bit)
            }

            // preamble mismatch, reset
            NrfDecoder::Sync(_, _) => NrfDecoder::Sync(1, bit),

            // receiving address
            NrfDecoder::RecvAddr(mut data, mut prefix) => {
                data.push(bit);
                match prefix.pop() {
                    // invalid prefix
                    Some(b) if b != bit => NrfDecoder::drop_bit_and_resync(config, out, data),
                    // insufficient length
                    _ if data.len() < config.address_len * 8 + config.padding_length_bits() => {
                        NrfDecoder::RecvAddr(data, prefix)
                    }
                    // fully received
                    _ => match config.payload_len {
                        Some(payload_len) if !config.shockburst => {
                            NrfDecoder::RecvPayload((config.address_len + payload_len) * 8, data)
                        }
                        _ => NrfDecoder::RecvHeader(data, config.length_header_bits_rev()),
                    },
                }
            }

            // receive ShockBurst header
            NrfDecoder::RecvHeader(mut data, mut length) => {
                data.push(bit);
                match length.pop() {
                    // invalid length
                    Some(b) if b != bit => NrfDecoder::drop_bit_and_resync(config, out, data),
                    _ if data.len() < (2 + config.address_len) * 8 => {
                        NrfDecoder::RecvHeader(data, length)
                    }
                    _ => {
                        let payload_len = data
                            .iter()
                            .skip(config.address_len * 8 + config.padding_length_bits())
                            .take(6)
                            .fold(0, |acc, bit| (acc << 1) | bit as usize);
                        if payload_len <= 32 {
                            NrfDecoder::RecvPayload(
                                (config.address_len + payload_len + 2) * 8,
                                data,
                            )
                        } else {
                            NrfDecoder::drop_bit_and_resync(config, out, data)
                        }
                    }
                }
            }

            // receiving data
            NrfDecoder::RecvPayload(message_len, mut data) => {
                data.push(bit);
                if data.len() < message_len {
                    NrfDecoder::RecvPayload(message_len, data)
                } else {
                    let crc = if config.shockburst { CRC16_SB } else { CRC16 };
                    let d = crc.checksum(&data.to_bytes());
                    NrfDecoder::CheckCrc(data, BitVec::from_bytes(&d.reverse_bits().to_be_bytes()))
                }
            }

            // checking CRC checksum
            NrfDecoder::CheckCrc(mut data, mut payload) => {
                data.push(bit);
                match payload.pop() {
                    // invalid checksum
                    Some(b) if b != bit => NrfDecoder::drop_bit_and_resync(config, out, data),
                    // crc checksum not fully received
                    _ if !payload.is_empty() => NrfDecoder::CheckCrc(data, payload),
                    // passed
                    _ => {
                        let mut payload = data.split_off(
                            config.padding_length_bits()
                                + config.address_len * 8
                                + config.header_length_bits(),
                        );
                        let mut address = data.split_off(config.padding_length_bits());
                        address.truncate(config.address_len * 8); // drop PCF
                        payload.truncate(payload.len() - 16); // drop CRC
                        out.push(
                            NrfFrame {
                                channel: config.channel,
                                address: address.to_bytes(),
                                payload: payload.to_bytes(),
                            },
                            &[],
                        );
                        NrfDecoder::Empty
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct NrfFrame {
    pub channel: u8,
    pub address: Vec<u8>,
    pub payload: Vec<u8>,
}

impl Display for NrfFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{:3} {} {}",
            self.channel,
            hex::encode(&self.address),
            hex::encode(&self.payload)
        )
    }
}

impl NrfFrame {
    #[cfg(test)]
    fn encode_fixed(&self) -> BitVec {
        let mut bits = BitVec::from_bytes(&self.address);
        bits.append(&mut BitVec::from_bytes(&self.payload));
        let crc = CRC16.checksum(&bits.to_bytes());
        bits.append(&mut BitVec::from_bytes(&crc.to_be_bytes()));
        for _ in 0..8 {
            bits.insert(0, !bits[0]);
        }
        bits
    }
    #[cfg(test)]
    fn encode_dynamic(&self) -> BitVec {
        let mut bits = BitVec::from_elem(7, false);
        bits.append(&mut BitVec::from_bytes(&self.address));
        bits.append(&mut BitVec::from_bytes(&[self.payload.len() as u8]).split_off(2));
        bits.grow(3, false);
        bits.append(&mut BitVec::from_bytes(&self.payload));
        let crc = CRC16_SB.checksum(&bits.to_bytes());
        bits.append(&mut BitVec::from_bytes(&crc.to_be_bytes()));
        bits = bits.split_off(7);
        for _ in 0..8 {
            bits.insert(0, !bits[0]);
        }
        bits
    }
}

pub struct NrfDeframer {
    src: Streamp<u8>,
    dst: NoCopyStreamp<NrfFrame>,
    config: NrfConfig,
    state: NrfDecoder,
}

impl NrfDeframer {
    pub fn new(src: Streamp<u8>, config: NrfConfig) -> Self {
        Self {
            src,
            dst: NoCopyStream::newp(),
            config,
            state: NrfDecoder::Empty,
        }
    }

    /// Get output stream.
    pub fn out(&self) -> NoCopyStreamp<NrfFrame> {
        self.dst.clone()
    }
}

impl Block for NrfDeframer {
    fn work(&mut self) -> Result<BlockRet, Error> {
        let ti = self.src.clone();
        let (input, _tags) = ti.read_buf()?;
        if input.is_empty() {
            return Ok(BlockRet::Noop);
        }

        self.state = input
            .iter()
            .copied()
            .map(|b| b != 0)
            .fold(self.state.clone(), |s, b| {
                s.push_bit(&self.config, &self.out(), b)
            });

        let n = input.len();
        input.consume(n);
        Ok(BlockRet::Ok)
    }
}

impl BlockName for NrfDeframer {
    fn block_name(&self) -> &str {
        "nRF deframer"
    }
}

impl BlockEOF for NrfDeframer {}

#[test]
fn test_fixed() {
    let packet = NrfFrame {
        channel: 39,
        address: vec![1, 2, 3, 4],
        payload: vec![5, 6, 7, 8, 9, 10],
    };
    let config = NrfConfig::fixed_length(39, packet.address.len(), packet.payload.len(), &[]);
    let out = NoCopyStream::newp();
    let mut state = NrfDecoder::Empty;
    for bit in packet.encode_fixed().iter() {
        state = state.push_bit(&config, &out, bit);
    }
    let (out, _) = out.pop().expect("Parsing failed");
    assert_eq!(out.address, packet.address);
    assert_eq!(out.payload, packet.payload);
}

#[test]
fn test_shockburst() {
    let packet = NrfFrame {
        channel: 39,
        address: vec![1, 2, 3, 4],
        payload: vec![5, 6, 7, 8, 9, 10],
    };
    let config = NrfConfig::shockburst(39, packet.address.len(), None, &[]);
    let out = NoCopyStream::newp();
    let mut state = NrfDecoder::Empty;
    for bit in packet.encode_dynamic().iter() {
        state = state.push_bit(&config, &out, bit);
    }
    let (out, _) = out.pop().expect("Parsing failed");
    assert_eq!(out.address, packet.address);
    assert_eq!(out.payload, packet.payload);
}
