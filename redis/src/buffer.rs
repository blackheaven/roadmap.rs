use bytes::Bytes;
use bytes::{Buf, BytesMut};
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

pub struct BufferedStream<Stream> {
    stream: Stream,
    buffer: BytesMut,
}

impl<Stream: AsyncReadExt + Unpin> BufferedStream<Stream> {
    pub fn new(stream: Stream) -> Self {
        Self {
            stream,
            buffer: BytesMut::with_capacity(1024 * 1024),
        }
    }

    pub async fn reset(&mut self) {
        self.buffer.clear();
    }

    pub async fn peek_u8(&mut self) -> Result<u8, std::io::Error> {
        if !self.buffer.has_remaining() {
            self.refill().await?
        }

        Ok(self.buffer.chunk()[0])
    }

    pub async fn get_u8(&mut self) -> Result<u8, std::io::Error> {
        if !self.buffer.has_remaining() {
            self.refill().await?
        }

        Ok(self.buffer.get_u8())
    }

    pub async fn skip(&mut self, n: usize) -> Result<(), std::io::Error> {
        while self.buffer.remaining() < n {
            self.refill().await?
        }

        self.buffer.advance(n);
        Ok(())
    }

    pub async fn get_u8_not_consume(&mut self, not: u8) -> Result<Option<u8>, std::io::Error> {
        let e = self.get_u8().await?;
        return Ok(if e == not { None } else { Some(e) });
    }

    pub async fn get_u8_until_consume(&mut self, not: u8) -> Result<Vec<u8>, std::io::Error> {
        let mut acc = Vec::with_capacity(64);
        while let Some(e) = self.get_u8_not_consume(not).await? {
            if acc.capacity() == acc.len() {
                acc.reserve(64);
            }
            acc.push(e);
        }
        return Ok(acc);
    }

    /// Read a new-line terminated decimal
    pub async fn get_decimal(&mut self) -> Result<u64, std::io::Error> {
        use atoi::atoi;

        let line = self.get_line().await?;

        atoi::<u64>(&line).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "protocol error; invalid frame format".to_string(),
            )
        })
    }

    pub async fn take(&mut self, n: usize) -> Result<Vec<u8>, std::io::Error> {
        while self.buffer.remaining() < n {
            self.refill().await?
        }

        let extract = Vec::from(self.buffer.chunk()[..n].to_vec());
        self.skip(n).await?;
        return Ok(extract);
    }

    /// Find a line
    pub async fn get_line(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let mut line = Vec::new();

        loop {
            line.extend(self.get_u8_until_consume(b'\r').await?);
            if let Some(e) = self.get_u8_not_consume(b'\n').await? {
                line.extend(vec![b'\r', e]);
            } else {
                return Ok(line);
            }
        }
    }

    async fn refill(&mut self) -> Result<(), std::io::Error> {
        if 0 == self.stream.read_buf(&mut self.buffer).await? {
            if !self.buffer.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "connection reset by peer".to_string(),
                ));
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "protocol error; not enough bytes".to_string(),
            ));
        }
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn advance_test() {
        let stream = tokio_stream::iter(vec![
            Result::<bytes::Bytes, std::io::Error>::Ok(Bytes::from_static(&[0, 1, 2, 3])),
            Result::<bytes::Bytes, std::io::Error>::Ok(Bytes::from_static(&[4, 5, 6, 7])),
            Result::<bytes::Bytes, std::io::Error>::Ok(Bytes::from_static(&[8, 9, 10, 11])),
        ]);

        // Convert it to an AsyncRead.
        let mut read = StreamReader::new(stream);
        let mut buffer = BufferedStream::new(read);

        println!("0");
        assert_eq!(buffer.get_u8().await.unwrap(), 0);
        println!("1");
        assert_eq!(buffer.get_u8().await.unwrap(), 1);
        println!("2");
        assert_eq!(buffer.get_u8().await.unwrap(), 2);
        println!("3");
        buffer.skip(8).await.unwrap();
        println!("4");
        assert_eq!(buffer.get_u8().await.unwrap(), 11);
        println!("5");
        assert_eq!(buffer.get_u8().await.is_err(), true);
        println!("6");
    }
}
