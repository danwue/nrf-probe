use crate::deframer::NrfFrame;
use itertools::Itertools;
use rustradio::block::{Block, BlockEOF, BlockName, BlockRet};
use rustradio::stream::NoCopyStreamp;
use rustradio::Error;
use std::collections::{HashMap, HashSet};

pub struct NrfStatSink {
    src: NoCopyStreamp<NrfFrame>,
    // history of received messages, containing count, channels and payload lengths
    history: HashMap<Vec<u8>, (usize, HashSet<u8>, HashSet<usize>)>,
}

impl NrfStatSink {
    pub fn new(src: NoCopyStreamp<NrfFrame>) -> Self {
        Self {
            src,
            history: HashMap::new(),
        }
    }
}

impl Block for NrfStatSink {
    fn work(&mut self) -> Result<BlockRet, Error> {
        let (v, _tags) = match self.src.pop() {
            None => return Ok(BlockRet::Noop),
            Some(x) => x,
        };

        match self.history.get_mut(&v.address) {
            Some((count, channel, payload_len)) => {
                *count += 1;
                channel.insert(v.channel);
                payload_len.insert(v.payload.len());
            }
            None => {
                self.history.insert(
                    v.address,
                    (
                        1,
                        HashSet::from([v.channel]),
                        HashSet::from([v.payload.len()]),
                    ),
                );
            }
        };

        print!("\x1B[2J\x1B[1;1H"); // reset terminal
        println!("Address    | Count | Payload Length | Channels");

        self.history
            .iter()
            .sorted_by_key(|(_, (count, _, _))| count)
            .rev()
            .take(10)
            .map(|(address, (count, channels, sizes))| {
                (
                    hex::encode(address),
                    count,
                    channels.iter().sorted().map(|s| s.to_string()).join(","),
                    sizes.iter().sorted().map(|s| s.to_string()).join(","),
                )
            })
            .for_each(|(addr, count, channels, sizes)| {
                println!("{:10} | {:5} | {:<14} | {}", addr, count, sizes, channels)
            });

        Ok(BlockRet::Ok)
    }
}

impl BlockName for NrfStatSink {
    fn block_name(&self) -> &str {
        "NrfStatSink"
    }
}

impl BlockEOF for NrfStatSink {}
