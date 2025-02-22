use std::io::{self, IoSlice};
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::{other_error, OtherErrorKind};
use crate::resultset::{MAX_POOL_ROWS, MAX_POOL_ROW_CAPACITY};

const U24_MAX: usize = 16_777_215;

pub struct PacketWriter<W> {
    pub seq: u8,
    w: W,
    queue: Vec<QueuedPacket>,

    /// Reusable packets
    preallocated: Vec<QueuedPacket>,
}

/// Type for packets being enqueued in the packet writer.
enum QueuedPacket {
    /// Raw queued packets are written as-is, these packets include header chunks.
    Raw(Arc<[u8]>),
    /// Packets constructed with headers are written as two IoSlices, the header and the body.
    WithHeader([u8; 4], Vec<u8>),
}

/// A helper function that performes a vector write to completion, since
/// the `tokio` one is not guranteed to write all of the data.
async fn write_all_vectored<'a, W: AsyncWrite + Unpin>(
    w: &'a mut W,
    mut slices: &'a mut [IoSlice<'a>],
) -> io::Result<()> {
    let mut n: usize = slices.iter().map(|s| s.len()).sum();

    loop {
        let mut did_write = w.write_vectored(slices).await?;

        if did_write == n {
            // Done, yay
            break Ok(());
        }

        n -= did_write;

        // Not done, need to advance the slices
        while did_write >= slices[0].len() {
            // First skip entire slices
            did_write -= slices[0].len();
            slices = &mut slices[1..];
        }

        // Skip a partial buffer
        slices[0].advance(did_write);
    }
}

// Gets an IoSlice to each of the packets currently enqueued in `queue`.
fn queued_packet_slices(queue: &[QueuedPacket]) -> Vec<IoSlice<'_>> {
    if queue.is_empty() {
        return Vec::new();
    }

    let mut slices = Vec::with_capacity(queue.len() * 2);
    queue.iter().for_each(|packet| match packet {
        QueuedPacket::WithHeader(hdr, pack) => {
            slices.push(IoSlice::new(hdr));
            slices.push(IoSlice::new(pack));
        }
        QueuedPacket::Raw(r) => {
            slices.push(IoSlice::new(r));
        }
    });

    slices
}

impl<W: AsyncWrite + Unpin> PacketWriter<W> {
    pub fn new(w: W) -> Self {
        PacketWriter {
            seq: 0,
            w,
            queue: Vec::new(),
            preallocated: Vec::new(),
        }
    }

    pub fn set_seq(&mut self, seq: u8) {
        self.seq = seq;
    }

    /// Flushes the writer. This function *must* be called before dropping the internal writer
    /// or writes may be lossed.
    pub async fn flush(&mut self) -> Result<(), tokio::io::Error> {
        self.write_queued_packets().await?;
        self.w.flush().await
    }

    /// Push a new packet to the outgoing packet list
    pub fn enqueue_packet(&mut self, mut packet: Vec<u8>) {
        // Lazily shrink large buffers before processing them further, as after that they will go to
        // the buffer pool
        packet.shrink_to(MAX_POOL_ROW_CAPACITY);

        while packet.len() >= U24_MAX {
            let rest = packet.split_off(U24_MAX);
            let mut hdr = (U24_MAX as u32).to_le_bytes();
            hdr[3] = self.seq;
            self.seq = self.seq.wrapping_add(1);
            self.queue.push(QueuedPacket::WithHeader(hdr, packet));
            packet = rest;
        }

        let mut hdr = (packet.len() as u32).to_le_bytes();
        hdr[3] = self.seq;
        self.seq = self.seq.wrapping_add(1);
        self.queue.push(QueuedPacket::WithHeader(hdr, packet));
    }

    /// Enqueues raw bytes to be written on the wire.
    pub async fn enqueue_raw(&mut self, packet: Arc<[u8]>) -> Result<(), tokio::io::Error> {
        self.queue.push(QueuedPacket::Raw(packet));
        Ok(())
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Send all the currently queued packets. Does not flush the writer.
    pub async fn write_queued_packets(&mut self) -> Result<(), tokio::io::Error> {
        let mut slices = queued_packet_slices(&self.queue);
        if !slices.is_empty() {
            write_all_vectored(&mut self.w, &mut slices).await?;
            self.return_queued_to_pool();
        }

        Ok(())
    }

    /// Handles split packet write (packets of 16MB and greater)
    async fn write_large_packet(&mut self, mut packet: &[u8]) -> Result<(), tokio::io::Error> {
        let mut slices = queued_packet_slices(&self.queue);

        // We need to prepare the headers in advance so we can borrow them later
        let mut total_len = packet.len();
        let mut headers = Vec::new();
        while total_len >= U24_MAX {
            let mut hdr = (U24_MAX as u32).to_le_bytes();
            hdr[3] = self.seq;
            self.seq = self.seq.wrapping_add(1);
            headers.push(hdr);
            total_len -= U24_MAX;
        }

        let mut hdr = (total_len as u32).to_le_bytes();
        hdr[3] = self.seq;
        self.seq = self.seq.wrapping_add(1);
        headers.push(hdr);

        // After the headers where computed we can issue a vectored write that references
        // both the headers and the packet slice, with no extra copying
        slices.reserve(headers.len() * 2);
        for header in &headers {
            slices.push(IoSlice::new(&header[..]));
            if packet.len() >= U24_MAX {
                let (first, rest) = packet.split_at(U24_MAX);
                slices.push(IoSlice::new(first));
                packet = rest;
            } else {
                slices.push(IoSlice::new(packet));
            }
        }

        write_all_vectored(&mut self.w, &mut slices).await?;
        self.return_queued_to_pool();

        Ok(())
    }

    /// Clear the queued packets and return them to the pool of preallocated packets
    fn return_queued_to_pool(&mut self) {
        // Prefer to merge the shorter vector into the longer vector, thus minimizing the amount of
        // copying neccessary. i.e. if `queue` already contains all the allocated vectors, no action
        // is needed.
        if self.queue.len() > self.preallocated.len() {
            std::mem::swap(&mut self.queue, &mut self.preallocated);
        }
        // Limit the number of pre allocated buffers to `MAX_POOL_ROWS`
        self.preallocated.truncate(MAX_POOL_ROWS);
        self.queue.truncate(MAX_POOL_ROWS - self.preallocated.len());
        self.preallocated.append(&mut self.queue);
    }

    /// Send a packet without queueing, flushes any queued packets beforehand
    pub async fn write_packet(&mut self, packet: &[u8]) -> Result<(), tokio::io::Error> {
        if packet.len() >= U24_MAX {
            return self.write_large_packet(packet).await;
        }

        let mut slices = queued_packet_slices(&self.queue);
        let packet_len = &packet.len().to_le_bytes()[0..3];
        let seq = &[self.seq];
        slices.extend([
            IoSlice::new(packet_len),
            IoSlice::new(seq),
            IoSlice::new(packet),
        ]);

        write_all_vectored(&mut self.w, &mut slices).await?;

        self.seq = self.seq.wrapping_add(1);
        Ok(())
    }

    pub fn get_buffer(&mut self) -> Vec<u8> {
        while let Some(p) = self.preallocated.pop() {
            match p {
                QueuedPacket::Raw(_) => {}
                QueuedPacket::WithHeader(_, mut vec) => {
                    vec.clear();
                    return vec;
                }
            }
        }
        Vec::new()
    }
}

pub struct PacketReader<R> {
    bytes: Vec<u8>,
    start: usize,
    remaining: usize,
    r: R,
}

impl<R> PacketReader<R> {
    pub fn new(r: R) -> Self {
        PacketReader {
            bytes: Vec::new(),
            start: 0,
            remaining: 0,
            r,
        }
    }
}

impl<R: AsyncRead + Unpin> PacketReader<R> {
    pub async fn next(&mut self) -> io::Result<Option<(u8, Packet<'_>)>> {
        self.start = self.bytes.len() - self.remaining;

        loop {
            if self.remaining != 0 {
                let bytes = {
                    // NOTE: this is all sorts of unfortunate. what we really want to do is to give
                    // &self.bytes[self.start..] to `packet()`, and the lifetimes should all work
                    // out. however, without NLL, borrowck doesn't realize that self.bytes is no
                    // longer borrowed after the match, and so can be mutated.
                    let bytes = &self.bytes.get(self.start..).ok_or_else(|| {
                        other_error(OtherErrorKind::IndexErr {
                            data: "self.bytes".to_string(),
                            index: self.start,
                            length: self.bytes.len(),
                        })
                    })?;
                    unsafe { ::std::slice::from_raw_parts(bytes.as_ptr(), bytes.len()) }
                };
                match packet(bytes) {
                    Ok((rest, p)) => {
                        self.remaining = rest.len();
                        return Ok(Some(p));
                    }
                    Err(nom::Err::Incomplete(_)) | Err(nom::Err::Error(_)) => {}
                    Err(nom::Err::Failure(ctx)) => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("{:?}", ctx),
                        ))
                    }
                }
            }

            // we need to read some more
            self.bytes.drain(0..self.start);
            self.start = 0;
            let end = self.bytes.len();
            let new_len = std::cmp::max(4096, end * 2);
            self.bytes.resize(new_len, 0);
            let read = {
                let buf = self.bytes.get_mut(end..).ok_or_else(|| {
                    other_error(OtherErrorKind::IndexErr {
                        data: "self.bytes".to_string(),
                        index: end,
                        length: new_len,
                    })
                })?;
                self.r.read(buf).await?
            };
            self.bytes.truncate(end + read);
            self.remaining = self.bytes.len();

            if read == 0 {
                if self.bytes.is_empty() {
                    return Ok(None);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!("{} unhandled bytes", self.bytes.len()),
                    ));
                }
            }
        }
    }
}

pub fn fullpacket(i: &[u8]) -> nom::IResult<&[u8], (u8, &[u8])> {
    let (i, _) = nom::bytes::complete::tag(&[0xff, 0xff, 0xff])(i)?;
    let (i, seq) = nom::bytes::complete::take(1u8)(i)?;
    let (i, bytes) = nom::bytes::complete::take(U24_MAX)(i)?;
    // nom::bytes::complete::take ensures that seq has one element
    #[allow(clippy::indexing_slicing)]
    Ok((i, (seq[0], bytes)))
}

pub fn onepacket(i: &[u8]) -> nom::IResult<&[u8], (u8, &[u8])> {
    let (i, length) = nom::number::complete::le_u24(i)?;
    let (i, seq) = nom::bytes::complete::take(1u8)(i)?;
    let (i, bytes) = nom::bytes::complete::take(length)(i)?;
    // nom::bytes::complete::take ensures that seq has one element
    #[allow(clippy::indexing_slicing)]
    Ok((i, (seq[0], bytes)))
}

// Clone because of https://github.com/Geal/nom/issues/1008
#[derive(Clone)]
pub struct Packet<'a>(&'a [u8], Vec<u8>);

impl<'a> Packet<'a> {
    fn extend(&mut self, bytes: &'a [u8]) {
        if self.0.is_empty() {
            if self.1.is_empty() {
                // first extend
                self.0 = bytes;
            } else {
                // later extend
                self.1.extend(bytes);
            }
        } else {
            assert!(self.1.is_empty());
            let mut v = self.0.to_vec();
            v.extend(bytes);
            self.1 = v;
            self.0 = &[];
        }
    }
}

impl<'a> AsRef<[u8]> for Packet<'a> {
    fn as_ref(&self) -> &[u8] {
        if self.1.is_empty() {
            self.0
        } else {
            &self.1
        }
    }
}

use std::ops::Deref;

impl<'a> Deref for Packet<'a> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

fn packet(i: &[u8]) -> nom::IResult<&[u8], (u8, Packet<'_>)> {
    nom::combinator::map(
        nom::sequence::pair(
            nom::multi::fold_many0(
                fullpacket,
                || (0, None),
                |(seq, pkt): (_, Option<Packet<'_>>), (nseq, p)| {
                    let pkt = if let Some(mut pkt) = pkt {
                        assert_eq!(nseq, seq + 1);
                        pkt.extend(p);
                        Some(pkt)
                    } else {
                        Some(Packet(p, Vec::new()))
                    };
                    (nseq, pkt)
                },
            ),
            onepacket,
        ),
        move |(full, last)| {
            let seq = last.0;
            let pkt = if let Some(mut pkt) = full.1 {
                assert_eq!(last.0, full.0 + 1);
                pkt.extend(last.1);
                pkt
            } else {
                Packet(last.1, Vec::new())
            };
            (seq, pkt)
        },
    )(i)
}

#[cfg(test)]
mod tests {
    use test_utils::slow;

    use super::*;

    #[test]
    fn test_one_ping() {
        assert_eq!(
            onepacket(&[0x01, 0, 0, 0, 0x10]).unwrap().1,
            (0, &[0x10][..])
        );
    }

    #[test]
    fn test_ping() {
        let p = packet(&[0x01, 0, 0, 0, 0x10]).unwrap().1;
        assert_eq!(p.0, 0);
        assert_eq!(&*p.1, &[0x10][..]);
    }

    #[test]
    fn test_long_exact() {
        let mut data = vec![0xff, 0xff, 0xff, 0];
        data.extend(&[0; U24_MAX][..]);
        data.push(0x00);
        data.push(0x00);
        data.push(0x00);
        data.push(1);

        let (rest, p) = packet(&data[..]).unwrap();
        assert!(rest.is_empty());
        assert_eq!(p.0, 1);
        assert_eq!(p.1.len(), U24_MAX);
        assert_eq!(&*p.1, &[0; U24_MAX][..]);
    }

    #[test]
    fn test_long_more() {
        let mut data = vec![0xff, 0xff, 0xff, 0];
        data.extend(&[0; U24_MAX][..]);
        data.push(0x01);
        data.push(0x00);
        data.push(0x00);
        data.push(1);
        data.push(0x10);

        let (rest, p) = packet(&data[..]).unwrap();
        assert!(rest.is_empty());
        assert_eq!(p.0, 1);
        assert_eq!(p.1.len(), U24_MAX + 1);
        assert_eq!(&p.1[..U24_MAX], &[0; U24_MAX][..]);
        assert_eq!(&p.1[U24_MAX..], &[0x10]);
    }

    #[tokio::test]
    #[slow]
    async fn test_large_packet_write() {
        let (u_out, u_in) = tokio::net::UnixStream::pair().unwrap();

        let packets = vec![
            vec![0u8; 245],
            vec![1u8; U24_MAX * 2],
            vec![2u8; U24_MAX + 100],
            vec![3u8; 100],
            vec![4u8; U24_MAX - 1],
            vec![5u8; U24_MAX],
        ];

        let p = packets.clone();
        tokio::spawn(async move {
            let mut writer = PacketWriter::new(u_out);

            for packet in &p {
                writer.enqueue_packet(packet.clone());
            }
            writer.write_queued_packets().await.unwrap();

            for packet in &p {
                writer.write_packet(&packet[..]).await.unwrap();
            }
            writer.flush().await.unwrap();
        });

        let mut reader = PacketReader::new(u_in);

        for _ in 0..2 {
            for encoded in &packets {
                let decoded = reader.next().await.unwrap().unwrap();
                assert_eq!(&decoded.1[..], encoded);
            }
        }

        assert!(reader.next().await.unwrap().is_none());
    }
}
