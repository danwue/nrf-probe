use rustradio::block::{Block, BlockEOF, BlockName, BlockRet};
use rustradio::stream::NoCopyStreamp;
use rustradio::Error;
use std::fmt::Display;

pub struct StdoutSink<T> {
    src: NoCopyStreamp<T>,
}

impl<T> StdoutSink<T> {
    pub fn new(src: NoCopyStreamp<T>) -> Self {
        Self { src }
    }
}

impl<T> Block for StdoutSink<T>
where
    T: Display,
{
    fn work(&mut self) -> Result<BlockRet, Error> {
        let (v, _tags) = match self.src.pop() {
            None => return Ok(BlockRet::Noop),
            Some(x) => x,
        };
        println!("{}", v);
        Ok(BlockRet::Ok)
    }
}

impl<T> BlockEOF for StdoutSink<T> {}

impl<T> BlockName for StdoutSink<T> {
    fn block_name(&self) -> &str {
        "StdoutSink"
    }
}
