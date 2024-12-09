use rustradio::block::{Block, BlockEOF, BlockName, BlockRet};
use rustradio::stream::{Stream, Streamp};
use rustradio::Error;

/// Multiply two streams, element by element
pub struct Multiply<T>
where
    T: Copy,
{
    a: Streamp<T>,
    b: Streamp<T>,
    dst: Streamp<T>,
}

impl<T> Multiply<T>
where
    T: Copy + std::ops::Mul<Output = T>,
{
    pub fn new(a: Streamp<T>, b: Streamp<T>) -> Self {
        Self {
            a,
            b,
            dst: Stream::newp(),
        }
    }

    pub fn out(&self) -> Streamp<T> {
        self.dst.clone()
    }
}

impl<T> Block for Multiply<T>
where
    T: Copy + std::ops::Mul<Output = T>,
{
    fn work(&mut self) -> Result<BlockRet, Error> {
        let (a, tags) = self.a.read_buf()?;
        let (b, _tags) = self.b.read_buf()?;
        let n = std::cmp::min(a.len(), b.len());
        if n == 0 {
            return Ok(BlockRet::Noop);
        }
        let mut o = self.dst.write_buf()?;
        let n = std::cmp::min(n, o.len());
        let it = a.iter().zip(b.iter()).map(|(x, y)| *x * *y);
        for (w, samp) in o.slice().iter_mut().take(n).zip(it) {
            *w = samp;
        }
        a.consume(n);
        b.consume(n);
        o.produce(n, &tags);
        Ok(BlockRet::Ok)
    }
}

impl<T> BlockEOF for Multiply<T> where T: Copy {}

impl<T> BlockName for Multiply<T>
where
    T: Copy,
{
    fn block_name(&self) -> &str {
        "Multiply"
    }
}
