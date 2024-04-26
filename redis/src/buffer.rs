// struct BufferedStream {
//     stream: BufWriter<TcpStream>,
//     buffer: BytesMut,
//     // position: u64,
// }
//
// impl BufferedStream {
//     pub fn new(socket: TcpStream) -> Self {
//         Self {
//             stream: BufWriter::new(socket),
//             buffer: BytesMut::with_capacity(1024 * 1024),
//             // position: 0,
//         }
//     }
//
//     // pub fn reset_position(&mut self) {
//     //     self.buffer.set_position(0);
//     //     self.refill();
//     // }
//
//     pub async fn peek_u8(&mut self) -> Result<u8, std::io::Error> {
//         if !self.buffer.has_remaining() {
//             self.refill()?
//         }
//
//         Ok(self.buffer.chunk()[0])
//     }
//
//     pub async fn get_u8(&mut self) -> Result<u8, std::io::Error> {
//         if !self.buffer.has_remaining() {
//             self.refill()?
//         }
//
//         Ok(self.buffer.get_u8())
//     }
//
//     pub async fn skip(&mut self, n: u64) -> Result<(), std::io::Error> {
//         if self.buffer.remaining() < n {
//             self.refill()?
//         }
//
//         self.buffer.advance(n);
//         Ok(())
//     }
//
//     /// Read a new-line terminated decimal
//     pub async fn get_decimal(&mut self) -> Result<u64, std::io::Error> {
//         use atoi::atoi;
//
//         let line = get_line(self.buffer)?;
//
//         atoi::<u64>(line).ok_or_else(|| "protocol error; invalid frame format".into())
//     }
//
//     /// Find a line
//     pub async fn get_line<'a>(&mut self) -> Result<&'a [u8], std::io::Error> {
//         // Scan the bytes directly
//         let start = self.buffer.position() as usize;
//         // Scan to the second to last byte
//         let end = self.buffer.len() - 1;
//
//         for i in start..end {
//             if self.buffer[i] == b'\r' && self.buffer[i + 1] == b'\n' {
//                 // We found a line, update the position to be *after* the \n
//                 self.skip((i + 2) as u64);
//
//                 // Return the line
//                 return Ok(&self.buffer[start..i]);
//             }
//         }
//
//         Err("pouet".into())
//     }
//
//     async fn refill(&mut self) -> Result<(), std::io::Error> {
//         if 0 == self.stream.read_buf(&mut self.buffer).await? {
//             if !self.buffer.is_empty() {
//                 return Err("connection reset by peer".to_string().into());
//             }
//         }
//         return Ok(());
//     }
// }
//
