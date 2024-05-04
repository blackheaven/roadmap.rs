use std::io;
use tokio::io::{AsyncWriteExt, BufWriter};

#[derive(Clone, Debug)]
pub enum Response {
    Stored,
    Quiet, // set xxx x x x noreply
    End,
    Value(String, u16, Vec<u8>),
}

impl Response {
    pub async fn write<T: AsyncWriteExt + Unpin>(
        &self,
        stream: &mut BufWriter<T>,
    ) -> io::Result<()> {
        match self {
            Response::Stored => {
                stream.write_all(b"STORED\r\n").await?;
            }
            Response::Quiet => {
                // No-op
            }
            Response::End => {
                stream.write_all(b"END\r\n").await?;
            }
            Response::Value(key, flags, data) => {
                use std::io::Write;

                // Convert the value to a string
                let mut buf = Vec::with_capacity(64);
                let mut buf = io::Cursor::new(&mut buf[..]);
                write!(&mut buf, "VALUE {} {} {} \r\n", key, flags, data.len())?;

                let pos = buf.position() as usize;
                stream.write_all(&buf.get_ref()[..pos]).await?;
                stream.write_all(data.as_ref()).await?;
                stream.write_all(b"END\r\n").await?;
            }
        }

        stream.flush().await
    }
}
