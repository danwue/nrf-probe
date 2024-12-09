use rustradio::block::{Block, BlockEOF, BlockName, BlockRet};
use rustradio::stream::{NoCopyStream, NoCopyStreamp};
use rustradio::Error;

pub struct Union<T> {
    a: NoCopyStreamp<T>,
    b: NoCopyStreamp<T>,
    dst: NoCopyStreamp<T>,
}

impl<T> Union<T> {
    pub fn new(a: NoCopyStreamp<T>, b: NoCopyStreamp<T>) -> Self {
        Self {
            a,
            b,
            dst: NoCopyStream::newp(),
        }
    }

    pub fn out(&self) -> NoCopyStreamp<T> {
        self.dst.clone()
    }
}

impl<T> Block for Union<T> {
    fn work(&mut self) -> Result<BlockRet, Error> {
        if let Some((val, tags)) = self.a.pop().or(self.b.pop()) {
            self.dst.push(val, &tags);
            Ok(BlockRet::Ok)
        } else {
            Ok(BlockRet::Noop)
        }
    }
}

impl<T> BlockEOF for Union<T> {
}

impl<T> BlockName for Union<T> {
    fn block_name(&self) -> &str {
        "Union"
    }
}
