/******************************************************************************
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.  If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/.
 * 
 * Software distributed under the License is distributed on an "AS IS" basis, 
 * WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for 
 * the specific language governing rights and limitations under the License.
 *
 * The Original Code is: gzip.rs
 * The Initial Developer of the Original Code is: William Wong (williamw520@gmail.com)
 * Portions created by William Wong are Copyright (C) 2013 William Wong, All Rights Reserved.
 *
 ******************************************************************************/

use std::str;
use std::num;
use std::vec;
use std::rt::io::{Reader, Writer, ReaderUtil, Decorator, ReaderByteConversions, WriterByteConversions};
use std::rt::io::{read_error, IoError, OtherIoError};
use std::rt::io::file::FileStream;
use std::rt::io::{Seek, SeekEnd};


use super::deflate;
use super::deflate::Deflator;
use super::deflate::Inflator;
use super::deflate::{DEFLATE_STATUS_OKAY, DEFLATE_STATUS_DONE, INFLATE_STATUS_DONE};


/// The buf_size_factor for internal IO buffers.
pub static MIN_SIZE_FACTOR : uint = 5;      // minimum size factor: 2^5 * 1K = 32K
pub static DEFAULT_SIZE_FACTOR : uint = 8;  // default size factor: 2^8 * 1K = 256K

/// The number of dictionary probes to use at each compression level (0-9). 0=implies fastest/minimal possible probing, 9=best compression but slowest
pub static MAX_COMPRESS_LEVEL : uint = 9;
pub static DEFAULT_COMPRESS_LEVEL : uint = 6;

static HEADER_FIXED_LEN: uint = 10;
static MAGIC1: u8 = 0x1f;
static MAGIC2: u8 = 0x8b;
static METHOD_DEFLATE: u8 = 8;

// Header flags
static FTEXT: u8    = 1;        // File is text file
static FHCRC: u8    = 2;        // Header CRC
static FEXTRA: u8   = 4;        // Extra field
static FNAME: u8    = 8;        // File name
static FCOMMENT: u8 = 16;       // File comment

static END_LENGTH: uint = 8;    // length of end section of a gzip file - 4 bytes CRC, 4 bytes original size



/// Calculate the IO buffer size in bytes given a buf_size_factor.
/// buf_size_factor is a power of 2.   buf_in_bytes = 1024 * 2 ^ buf_size_factor
pub fn calc_buf_size(buf_size_factor: uint) -> uint {
    return deflate::calc_buf_size(buf_size_factor);
}



/// GZip structure for tracking gzip compression and decompression
pub struct GZip {
    // Header fields
    id1:            u8,
    id2:            u8,
    compression:    u8,
    flags:          u8,
    mtime:          u32,
    xflags:         u8,
    os:             u8,
    xfield_len:     Option<u16>,
    xfield:         Option<~[u8]>,
    filename:       Option<~str>,
    comment:        Option<~str>,
    header_crc:     Option<u16>,

    // End section
    crc32:          u32,
    original_size:  u32,

    // Misc
    cmp_crc32:      u32,
}

impl GZip {

    /// Initialize a new GZip structure for decompression.  Read in the gzip header.
    /// Return the new GZip structure.
    pub fn decompress_init<R: Reader>(reader: &mut R) -> Result<GZip, ~str> {
        let mut gzip = GZip::new();
        match gzip.readHeader(reader).and_then( |_| gzip.readHeaderExtra(reader) ) {
            Ok(_)   => Ok(gzip),
            Err(s)  => Err(s)
        }
    }

    /// Initialize a new GZip structure for compression.  Write out the gzip header.
    /// Return the new GZip structure.
    pub fn compress_init<W: Writer>(writer: &mut W, file_name: &str, mtime: u32, file_size: u32) -> Result<GZip, ~str> {
        let mut gzip = GZip::new();
        gzip.mtime = mtime;
        gzip.filename = if file_name.len() > 0 { Some(file_name.to_owned()) } else { None };
        gzip.flags = if gzip.filename.is_some() { FNAME } else { 0 };
        gzip.original_size = file_size;
        match gzip.writeHeader(writer).and_then(|_| gzip.writeHeaderExtra(writer)) {
            Ok(_)   => Ok(gzip),
            Err(s)  => Err(s)
        }
    }

    pub fn read_info(file_reader: &mut FileStream) -> Result<GZip, ~str> {
        let mut gzip = GZip::new();
        match gzip.readHeader(file_reader).and_then( |_| gzip.readHeaderExtra(file_reader) ) {
            Ok(_)   => {
                file_reader.seek(-END_LENGTH as i64, SeekEnd);
                let mut end_buf = [0u8, ..END_LENGTH];
                read_buf_upto(file_reader, end_buf, 0, END_LENGTH);
                match gzip.readEndSection(end_buf, end_buf.len()) {
                    Ok(_)   => Ok(gzip),
                    Err(s)  => Err(s)
                }
            },
            Err(s)  => Err(s)
        }
    }

    fn new() -> GZip {
        GZip {
            id1:            MAGIC1,
            id2:            MAGIC2,
            compression:    METHOD_DEFLATE,
            flags:          0,
            mtime:          0,
            xflags:         0,
            os:             0,
            xfield_len:     None,
            xfield:         None,
            filename:       None,
            comment:        None,
            header_crc:     None,
            crc32:          0,
            original_size:  0,
            cmp_crc32:      0,
        }
    }


    /// Decompress the data read from the reader and pipe the output to writer directly.
    /// Read the compressed data from reader, decompress them, and output them to writer, in an internal loop.
    /// Runs until reading EOF from reader.  This is more efficient than GZipReader.
    /// Requires decompress_init() to be called first.
    /// buf_size_factor is used for internal IO buffers, with MIN_SIZE_FACTOR.  It is the power in 2.
    pub fn decompress_pipe<R: Reader, W: Writer>(&mut self, reader: &mut R, writer: &mut W, buf_size_factor: uint) -> Result<~[u8], ~str> {
        let mut extra_buf = ~[];
        let mut end_buf = [0u8, ..END_LENGTH];
        let mut end_len = 0u;
        let mut inflator = Inflator::with_size_factor(buf_size_factor);
        let status = inflator.decompress_pipe(
            // upcall function to read input data for decompression
            |in_buf| {
                let read_len = if reader.eof() {
                    0                           // EOF
                } else {
                    match reader.read(in_buf) {
                        Some(nread) => nread,   // read number of bytes read, including 0 for EOF;
                        None => 0               // EOF
                    }
                };
                read_len
            },
            // upcall function to write the decompressed data
            |out_buf, is_eof| {
                self.cmp_crc32 = update_crc(self.cmp_crc32, out_buf, 0, out_buf.len());     // compute the CRC on the decompressed data
                writer.write(out_buf);
                if is_eof {
                    writer.flush();
                }
                false                           // don't abort
            },
            // upcall function to handle the remaining input data that are not part of the compressed data.
            |rest_buf| {
                // Move the rest of the bytes into end_buf, and read more into end_buf if not enough bytes for it.
                end_len = rest_buf.len();
                let copy_len = num::min(END_LENGTH, end_len);
                vec::bytes::copy_memory(end_buf, rest_buf, copy_len);
                extra_buf.push_all(rest_buf.slice_from(copy_len));  // Move anything beyond the gzip end section into extra_buf.
                if end_len < END_LENGTH {                           // Read in the rest of end section if not enough data in rest_buf
                    end_len += read_buf_upto(reader, end_buf, end_len, END_LENGTH - end_len);
                }
            } );
        inflator.free();

        match status {
            INFLATE_STATUS_DONE => {
                match self.readEndSection(end_buf, end_len)
                    .and_then( |_| self.checkCrc() ) {
                    Ok(_)   => Ok(extra_buf),                       // Return the extra bytes beyond the end of gzip data.
                    Err(s)  => Err(s)
                }
            },
            _ => 
                Err(fmt!("Failed to decompress data.  Status: %?", status))
        }
    }

    fn readHeader<R: Reader>(&mut self, reader: &mut R) -> Result<uint, ~str> {

        let mut buf = [0, ..HEADER_FIXED_LEN];
        if read_buf_upto(reader, buf, 0, HEADER_FIXED_LEN) != HEADER_FIXED_LEN {
            return Err(~"Too few data to be a valid gzip format.");
        }

        self.id1 = buf[0];
        self.id2 = buf[1];
        self.compression = buf[2];
        self.flags = buf[3];
        self.mtime = unpack_u32_le(buf, 4);
        self.xflags = buf[8];
        self.os = buf[9];

        if self.id1 != MAGIC1 || self.id2 != MAGIC2 {
            return Err(~"Invalid gzip signature");
        }
        if self.compression != METHOD_DEFLATE {
            return Err(~"Unsupported compression method");
        }

        Ok(0)
    }

    fn readHeaderExtra<R: Reader>(&mut self, reader: &mut R) -> Result<uint, ~str> {

        if (self.flags & FEXTRA) == FEXTRA {
            self.xfield_len = Some(reader.read_le_u16_());
            let xf_len = self.xfield_len.unwrap() as uint;
            let mut buf = vec::from_elem(xf_len, 0u8);
            read_buf_upto(reader, buf, 0, xf_len);
            self.xfield = Some(buf);
        }

        if (self.flags & FNAME) == FNAME {
            self.filename = Some(read_strz(reader));
        }

        if (self.flags & FCOMMENT) == FCOMMENT {
            self.comment = Some(read_strz(reader));
        }

        if (self.flags & FHCRC) == FHCRC {
            self.header_crc = Some(reader.read_le_u16_());
        }

        Ok(0)
    }

    fn readEndSection(&mut self, end_buf: &[u8], end_len: uint) -> Result<uint, ~str> {
        if end_len < END_LENGTH {
            return Err(fmt!("Not enough data in gzip end section.  Bytes missing: %?", (END_LENGTH - end_len)));
        }
        self.crc32 = unpack_u32_le(end_buf, 0);
        self.original_size = unpack_u32_le(end_buf, 4);
        Ok(0)
    }

    fn checkCrc(&mut self) -> Result<uint, ~str> {
        if self.crc32 != self.cmp_crc32 {
            return Err(~"The computed CRC of the decompressed data does not match the stored CRC in the file.")
        }
        Ok(0)
    }


    /// Compress the data read from the reader and pipe the output to writer directly.
    /// Read the plain data from reader, compress them, and output them to writer, in an internal loop.
    /// Runs until reading EOF from reader.  This is more efficient than GZipWriter.
    /// Requires compress_init() to be called first.
    /// compress_level is 0-9 for faster but lower compression ratio to slower but higher compression ratio.
    /// Control the internal IO buffer size with buf_size_factor.  See calc_buf_size() for the actual bytes computed.
    /// buf_size_factor is used for internal IO buffers, with MIN_SIZE_FACTOR.  It is the power in 2.
    pub fn compress_pipe<R: Reader, W: Writer>(&mut self, reader: &mut R, writer: &mut W, compress_level: uint, buf_size_factor: uint) -> Result<uint, ~str> {
        let mut deflator = Deflator::with_size_factor(buf_size_factor);
        deflator.init(compress_level, false, false);
        let status = deflator.compress_pipe(
            // upcall function to read input data for compression
            |in_buf| {
                if reader.eof() {
                    0                           // EOF
                } else {
                    match reader.read(in_buf) {
                        Some(nread) => {
                            self.cmp_crc32 = update_crc(self.cmp_crc32, in_buf, 0, nread);
                            nread               // read number of bytes read, including 0 for EOF
                        },
                        None => 0               // EOF
                    }
                }
            },
            // upcall function to write the decompressed data
            |out_buf, is_eof| {
                writer.write(out_buf);
                if is_eof {
                    writer.flush();
                }
                false                           // don't abort
            });
        deflator.free();
    
        match status {
            DEFLATE_STATUS_DONE => {
                self.crc32 = self.cmp_crc32;
                self.writeEndSection(writer);
                Ok(0)
            },
            _ => 
                Err(fmt!("Failed to compress data.  Status: %?", status))
        }
    }

    fn writeHeader<W: Writer>(&self, writer: &mut W) -> Result<uint, ~str> {

        let mut buf = [0, ..HEADER_FIXED_LEN];

        buf[0] = self.id1;
        buf[1] = self.id2;
        buf[2] = self.compression;
        buf[3] = self.flags;
        pack_u32_le(buf, 4, self.mtime);
        buf[8] = self.xflags;
        buf[9] = self.os;

        writer.write(buf);
        Ok(0)
    }

    fn writeHeaderExtra<W: Writer>(&self, writer: &mut W) -> Result<uint, ~str> {

        if (self.flags & FEXTRA) == FEXTRA {
            writer.write_le_u16_(self.xfield_len.unwrap());
            writer.write(self.xfield.clone().unwrap());
        }

        if (self.flags & FNAME) == FNAME {
            let buf = to_strz(self.filename.clone().unwrap());
            writer.write(buf);
        }

        if (self.flags & FCOMMENT) == FCOMMENT {
            let buf = to_strz(self.comment.clone().unwrap());
            writer.write(buf);
        }

        if (self.flags & FHCRC) == FHCRC {
            writer.write_le_u16_(self.header_crc.unwrap());
        }

        Ok(0)
    }

    fn writeEndSection<W: Writer>(&self, writer: &mut W) {
        let mut end_buf = [0, ..END_LENGTH];

        pack_u32_le(end_buf, 0, self.crc32);
        pack_u32_le(end_buf, 4, self.original_size);
        writer.flush();
        writer.write(end_buf);
        writer.flush();
    }

}


pub struct GZipReader<R> {
    gzip:           GZip,
    inner_reader:   R,
    inflator:       Inflator,
    is_eof:         bool,
}

/// Decorator to access the inner reader
impl<R: Reader> Decorator<R> for GZipReader<R> {
    fn inner(self) -> R {
        self.inner_reader
    }

    fn inner_ref<'a>(&'a self) -> &'a R {
        &self.inner_reader
    }

    fn inner_mut_ref<'a>(&'a mut self) -> &'a mut R {
        &mut self.inner_reader
    }
}

impl<R: Reader> GZipReader<R> {

    /// Create a GZipReader to decompress data from the inner_reader automatically when reading.
    pub fn new(inner_reader: R) -> Result<GZipReader<R>, ~str> {
        GZipReader::with_size_factor(inner_reader, DEFAULT_SIZE_FACTOR)
    }

    /// Create a GZipReader to decompress data from the inner_reader automatically when reading.
    /// Control the internal IO buffer size with buf_size_factor.  See calc_buf_size() for the actual bytes computed.
    /// buf_size_factor is used for internal IO buffers, with MIN_SIZE_FACTOR.  It is the power in 2.
    pub fn with_size_factor(mut inner_reader: R, buf_size_factor: uint) -> Result<GZipReader<R>, ~str> {
        match GZip::decompress_init(&mut inner_reader) {
            Ok(gzip) => {
                Ok(GZipReader {
                        gzip:           gzip,
                        inner_reader:   inner_reader,
                        inflator:       Inflator::with_size_factor(buf_size_factor),
                        is_eof:         false,
                    })
            },
            Err(s) => 
                Err(s)
        }
    }
}

impl<R: Reader> Reader for GZipReader<R> {
    /// Read the decompressed data from the inner_reader automatically.
    fn read(&mut self, output_buf: &mut [u8]) -> Option<uint> {
        let mut end_buf = [0u8, ..END_LENGTH];
        let mut end_len;

        let status = self.inflator.decompress_read(
            // Callback to read input data.
            |in_buf| {
                if self.inner_reader.eof() {
                    0                           // Return 0 for EOF
                } else {
                    match self.inner_reader.read(in_buf) {
                        Some(nread) => nread,   // Return number of bytes read, including 0 for EOF
                        None => 0               // REturn 0 for EOF
                    }
                }
            },
            output_buf);

        match status {
            Ok(0) => {
                self.is_eof = true;
                // Move the rest of the bytes into end_buf, and read more into end_buf if not enough bytes for it.
                end_len = self.inflator.get_rest(end_buf);
                if end_len < END_LENGTH {
                    end_len += read_buf_upto(&mut self.inner_reader, end_buf, end_len, END_LENGTH - end_len);
                }
                match self.gzip.readEndSection(end_buf, end_len)
                    .and_then( |_| self.gzip.checkCrc() ) {
                    Ok(_)   => return None,
                    Err(e)  => {
                        read_error::cond.raise(IoError {
                                kind: OtherIoError,
                                desc: "Read failure in decompression",
                                detail: Some(fmt!("Failure in reading end section.  %?", e))
                            });
                        None
                    }
                }
            },
            Ok(output_len) => {
                self.gzip.cmp_crc32 = update_crc(self.gzip.cmp_crc32, output_buf, 0, output_len);
                return Some(output_len);
            },
            _ => {
                read_error::cond.raise(IoError {
                        kind: OtherIoError,
                        desc: "Read failure in decompression",
                        detail: Some(fmt!("Read failure in deflate::decompress_read().  status: %?", status))
                    });
                None
            }
        }
    }

    fn eof(&mut self) -> bool {
        return self.is_eof;
    }
}



pub struct GZipWriter<W> {
    gzip:           GZip,
    inner_writer:   W,
    deflator:       Deflator,
    finalized:      bool,
}

impl<W: Writer> GZipWriter<W> {

    /// Create a GZipWriter to compress data automatically when writing.
    /// file_name is the original filename to store in the gzip file.
    /// mtime is the original modified time to store in the gzip file.
    /// file_size is the original file size to store in the gzip file.
    pub fn new(inner_writer: W, file_name: &str, mtime: u32, file_size: u32) -> Result<GZipWriter<W>, ~str> {
        GZipWriter::with_size_factor(inner_writer, file_name, mtime, file_size, DEFAULT_COMPRESS_LEVEL, DEFAULT_SIZE_FACTOR)
    }

    /// Create a GZipWriter to compress data automatically when writing.
    /// The compress_level (0-9) is a trade off in compression ratio vs compression speed.
    /// Control the internal IO buffer size with buf_size_factor.  See calc_buf_size() for the actual bytes computed.
    /// buf_size_factor is used for internal IO buffers, with MIN_SIZE_FACTOR.  It is the power in 2.
    pub fn with_size_factor(mut inner_writer: W, file_name: &str, mtime: u32, file_size: u32, compress_level: uint, buf_size_factor: uint) -> Result<GZipWriter<W>, ~str> {
        match GZip::compress_init(&mut inner_writer, file_name, mtime, file_size) {
            Ok(gzip) => {
                let deflator = Deflator::with_size_factor(buf_size_factor);
                deflator.init(compress_level, false, false);
                Ok(GZipWriter {
                        gzip:           gzip,
                        inner_writer:   inner_writer,
                        deflator:       deflator,
                        finalized:      false,
                    })
            },
            Err(s) => 
                Err(s)
        }
    }

    /// Finalize the compression stream and flush out any pending compressed data.
    /// The caller must call this at the end of writing data into this writer to compress.
    /// After this is called, this writer cannot be written again.
    pub fn finalize(&mut self) {
        if self.finalized {
            return;
        }
        // Do a final_write to finalize the compression session and flush out the remaining compressed data.
        let output_buf = [0u8, ..0];
        self.do_write(output_buf, true);
    }

    fn do_write(&mut self, output_buf: &[u8], final_write: bool) {
        if self.finalized {
            read_error::cond.raise(IoError {
                    kind: OtherIoError,
                    desc: "Writing on a closed stream",
                    detail: Some(~"The compression stream has been closed."),
                });
        }

        self.gzip.cmp_crc32 = update_crc(self.gzip.cmp_crc32, output_buf, 0, output_buf.len());
        let status = self.deflator.compress_write(output_buf, final_write, |out_buf, is_eof| {
                // Callback to write the compressed data.
                self.inner_writer.write(out_buf);
                if is_eof {
                    self.inner_writer.flush();
                }
            });
        match status {
            DEFLATE_STATUS_OKAY => {
            },
            DEFLATE_STATUS_DONE => {
                self.gzip.crc32 = self.gzip.cmp_crc32;
                self.gzip.writeEndSection(&mut self.inner_writer);
                self.finalized = true;
            },
            _ => {
                read_error::cond.raise(IoError {
                        kind: OtherIoError,
                        desc: "Write failure in compression",
                        detail: Some(fmt!("Failure in compressing data.  %?", status))
                    });
            }
        }
    }

}

impl<W: Writer> Writer for GZipWriter<W> {

    fn write(&mut self, output_buf: &[u8]) {
        self.do_write(output_buf, false);
    }

    fn flush(&mut self) {
        return self.inner_writer.flush();
    }
}

/// Decorator to access the inner writer
impl<W: Writer> Decorator<W> for GZipWriter<W> {
    fn inner(self) -> W {
        self.inner_writer
    }

    fn inner_ref<'a>(&'a self) -> &'a W {
        &self.inner_writer
    }

    fn inner_mut_ref<'a>(&'a mut self) -> &'a mut W {
        &mut self.inner_writer
    }
}


/// Pack a u32 into byte buffer in little-endian
fn pack_u32_le(buf: &mut [u8], offset: uint, value: u32) -> uint {
    buf[offset + 0] = (value >> 0) as u8;
    buf[offset + 1] = (value >> 8) as u8;
    buf[offset + 2] = (value >> 16) as u8;
    buf[offset + 3] = (value >> 24) as u8;
    offset + 4
}

/// Unpack a u32 from byte buffer in little-endian
fn unpack_u32_le(buf: &[u8], offset: uint) -> u32 {
    ( ((buf[offset + 0] as u32) & 0xFF)       ) |
    ( ((buf[offset + 1] as u32) & 0xFF) << 8  ) |
    ( ((buf[offset + 2] as u32) & 0xFF) << 16 ) |
    ( ((buf[offset + 3] as u32) & 0xFF) << 24 )
}

fn to_strz(str_value: &str) -> ~[u8] {
    let str_bytes = str_value.as_bytes();
    let mut buf = vec::from_elem(str_bytes.len() + 1, 0u8);
    vec::bytes::copy_memory(buf, str_bytes, str_bytes.len());
    buf[buf.len() - 1] = 0;
    return buf;
}

// Read a zero-terminated str.  Read until encountering the terminating 0.
fn read_strz<R: Reader>(reader: &mut R) -> ~str {
    let mut buf = ~[];
    loop {
        match reader.read_byte() {
            Some(0)     => break,
            Some(ch)    => buf.push(ch),
            None        => break
        }
    }
    return str::from_utf8(buf);
}

fn read_buf_upto<R: Reader>(reader: &mut R, buf: &mut [u8], offset: uint, len_to_read: uint) -> uint {
    let mut total_read = 0u;
    while total_read < len_to_read {
        let remaining_len = len_to_read - total_read;
        let begin = offset + total_read;
        let end   = offset + total_read + remaining_len;
        let slice_buf = buf.mut_slice(begin, end);
        match reader.read(slice_buf) {
            Some(read_len) => total_read = total_read + read_len,
            None => break
        }
    }
    return total_read;
}


fn compute_crc(buf: &[u8], from: uint, to: uint) -> u32 {
    // Seed CRC with 0
    return update_crc(0u32, buf, from, to);
}

fn update_crc(mut crc: u32, buf: &[u8], from: uint, to: uint) -> u32 {
    crc = crc ^ 0xFFFFFFFF;     // Pre one's complement;
    for n in range(from, to) {
        crc = crc_table[(crc ^ buf[n] as u32) & 0xff] ^ (crc >> 8);
    }
    return crc ^ 0xFFFFFFFF;    // Post one's complement
}


// Make CRC table according to gzip spec.
fn make_crc_table() -> [u32, ..256] {
    let mut table = [0u32, ..256];
    let mut c : u32;

    for n in range(0, 256) {
        c = n as u32;
        for _ in range(0, 8) {
            if c & 1 == 1 {
                c = 0xedb88320u32 ^ (c >> 1);
            } else {
                c = c >> 1;
            }
        }
        table[n] = c;
    }
    table
}

// Run this to pre-generate the CRC table to be included in source code.
pub fn generate_crc_table() {
    let table = make_crc_table();
    let mut output = ~"static crc_table : [u32, ..256] = [";
    for n in range(0, 256) {
        if n % 8 == 0 {
            output = output + "\n    ";
        }
        output = output + fmt!("0x%Xu32, ", table[n] as uint);
    }
    output = output + "\n];";
    println(output);
}

// Copied from the generated code from the above function
static crc_table : [u32, ..256] = [
    0x0u32, 0x77073096u32, 0xEE0E612Cu32, 0x990951BAu32, 0x76DC419u32, 0x706AF48Fu32, 0xE963A535u32, 0x9E6495A3u32,
    0xEDB8832u32, 0x79DCB8A4u32, 0xE0D5E91Eu32, 0x97D2D988u32, 0x9B64C2Bu32, 0x7EB17CBDu32, 0xE7B82D07u32, 0x90BF1D91u32,
    0x1DB71064u32, 0x6AB020F2u32, 0xF3B97148u32, 0x84BE41DEu32, 0x1ADAD47Du32, 0x6DDDE4EBu32, 0xF4D4B551u32, 0x83D385C7u32,
    0x136C9856u32, 0x646BA8C0u32, 0xFD62F97Au32, 0x8A65C9ECu32, 0x14015C4Fu32, 0x63066CD9u32, 0xFA0F3D63u32, 0x8D080DF5u32,
    0x3B6E20C8u32, 0x4C69105Eu32, 0xD56041E4u32, 0xA2677172u32, 0x3C03E4D1u32, 0x4B04D447u32, 0xD20D85FDu32, 0xA50AB56Bu32,
    0x35B5A8FAu32, 0x42B2986Cu32, 0xDBBBC9D6u32, 0xACBCF940u32, 0x32D86CE3u32, 0x45DF5C75u32, 0xDCD60DCFu32, 0xABD13D59u32,
    0x26D930ACu32, 0x51DE003Au32, 0xC8D75180u32, 0xBFD06116u32, 0x21B4F4B5u32, 0x56B3C423u32, 0xCFBA9599u32, 0xB8BDA50Fu32,
    0x2802B89Eu32, 0x5F058808u32, 0xC60CD9B2u32, 0xB10BE924u32, 0x2F6F7C87u32, 0x58684C11u32, 0xC1611DABu32, 0xB6662D3Du32,
    0x76DC4190u32, 0x1DB7106u32, 0x98D220BCu32, 0xEFD5102Au32, 0x71B18589u32, 0x6B6B51Fu32, 0x9FBFE4A5u32, 0xE8B8D433u32,
    0x7807C9A2u32, 0xF00F934u32, 0x9609A88Eu32, 0xE10E9818u32, 0x7F6A0DBBu32, 0x86D3D2Du32, 0x91646C97u32, 0xE6635C01u32,
    0x6B6B51F4u32, 0x1C6C6162u32, 0x856530D8u32, 0xF262004Eu32, 0x6C0695EDu32, 0x1B01A57Bu32, 0x8208F4C1u32, 0xF50FC457u32,
    0x65B0D9C6u32, 0x12B7E950u32, 0x8BBEB8EAu32, 0xFCB9887Cu32, 0x62DD1DDFu32, 0x15DA2D49u32, 0x8CD37CF3u32, 0xFBD44C65u32,
    0x4DB26158u32, 0x3AB551CEu32, 0xA3BC0074u32, 0xD4BB30E2u32, 0x4ADFA541u32, 0x3DD895D7u32, 0xA4D1C46Du32, 0xD3D6F4FBu32,
    0x4369E96Au32, 0x346ED9FCu32, 0xAD678846u32, 0xDA60B8D0u32, 0x44042D73u32, 0x33031DE5u32, 0xAA0A4C5Fu32, 0xDD0D7CC9u32,
    0x5005713Cu32, 0x270241AAu32, 0xBE0B1010u32, 0xC90C2086u32, 0x5768B525u32, 0x206F85B3u32, 0xB966D409u32, 0xCE61E49Fu32,
    0x5EDEF90Eu32, 0x29D9C998u32, 0xB0D09822u32, 0xC7D7A8B4u32, 0x59B33D17u32, 0x2EB40D81u32, 0xB7BD5C3Bu32, 0xC0BA6CADu32,
    0xEDB88320u32, 0x9ABFB3B6u32, 0x3B6E20Cu32, 0x74B1D29Au32, 0xEAD54739u32, 0x9DD277AFu32, 0x4DB2615u32, 0x73DC1683u32,
    0xE3630B12u32, 0x94643B84u32, 0xD6D6A3Eu32, 0x7A6A5AA8u32, 0xE40ECF0Bu32, 0x9309FF9Du32, 0xA00AE27u32, 0x7D079EB1u32,
    0xF00F9344u32, 0x8708A3D2u32, 0x1E01F268u32, 0x6906C2FEu32, 0xF762575Du32, 0x806567CBu32, 0x196C3671u32, 0x6E6B06E7u32,
    0xFED41B76u32, 0x89D32BE0u32, 0x10DA7A5Au32, 0x67DD4ACCu32, 0xF9B9DF6Fu32, 0x8EBEEFF9u32, 0x17B7BE43u32, 0x60B08ED5u32,
    0xD6D6A3E8u32, 0xA1D1937Eu32, 0x38D8C2C4u32, 0x4FDFF252u32, 0xD1BB67F1u32, 0xA6BC5767u32, 0x3FB506DDu32, 0x48B2364Bu32,
    0xD80D2BDAu32, 0xAF0A1B4Cu32, 0x36034AF6u32, 0x41047A60u32, 0xDF60EFC3u32, 0xA867DF55u32, 0x316E8EEFu32, 0x4669BE79u32,
    0xCB61B38Cu32, 0xBC66831Au32, 0x256FD2A0u32, 0x5268E236u32, 0xCC0C7795u32, 0xBB0B4703u32, 0x220216B9u32, 0x5505262Fu32,
    0xC5BA3BBEu32, 0xB2BD0B28u32, 0x2BB45A92u32, 0x5CB36A04u32, 0xC2D7FFA7u32, 0xB5D0CF31u32, 0x2CD99E8Bu32, 0x5BDEAE1Du32,
    0x9B64C2B0u32, 0xEC63F226u32, 0x756AA39Cu32, 0x26D930Au32, 0x9C0906A9u32, 0xEB0E363Fu32, 0x72076785u32, 0x5005713u32,
    0x95BF4A82u32, 0xE2B87A14u32, 0x7BB12BAEu32, 0xCB61B38u32, 0x92D28E9Bu32, 0xE5D5BE0Du32, 0x7CDCEFB7u32, 0xBDBDF21u32,
    0x86D3D2D4u32, 0xF1D4E242u32, 0x68DDB3F8u32, 0x1FDA836Eu32, 0x81BE16CDu32, 0xF6B9265Bu32, 0x6FB077E1u32, 0x18B74777u32,
    0x88085AE6u32, 0xFF0F6A70u32, 0x66063BCAu32, 0x11010B5Cu32, 0x8F659EFFu32, 0xF862AE69u32, 0x616BFFD3u32, 0x166CCF45u32,
    0xA00AE278u32, 0xD70DD2EEu32, 0x4E048354u32, 0x3903B3C2u32, 0xA7672661u32, 0xD06016F7u32, 0x4969474Du32, 0x3E6E77DBu32,
    0xAED16A4Au32, 0xD9D65ADCu32, 0x40DF0B66u32, 0x37D83BF0u32, 0xA9BCAE53u32, 0xDEBB9EC5u32, 0x47B2CF7Fu32, 0x30B5FFE9u32,
    0xBDBDF21Cu32, 0xCABAC28Au32, 0x53B39330u32, 0x24B4A3A6u32, 0xBAD03605u32, 0xCDD70693u32, 0x54DE5729u32, 0x23D967BFu32,
    0xB3667A2Eu32, 0xC4614AB8u32, 0x5D681B02u32, 0x2A6F2B94u32, 0xB40BBE37u32, 0xC30C8EA1u32, 0x5A05DF1Bu32, 0x2D02EF8Du32,
];


#[cfg(test)]
mod tests {

    use std::rt::io::Reader;
    use std::rt::io::mem::MemReader;
    use std::rand;
    use std::rand::Rng;
    use super::*;

    #[test]
    fn test_generate_crc_table() {
        // Uncomment to generate the crc table text.
        //generate_crc_table();
    }

    #[test]
    fn test_gzip_reader() {

        let comp_reader = MemReader::new(~[0x1f, 0x8B, 0x08, 0x08, 0x54, 0x3C, 0x3D, 0x52, 0x00, 0x03, 0x74, 0x65, 0x73, 0x74, 0x31, 0x00, 0x73, 0x74, 0x72, 0x76, 0x71, 0x75, 0x73, 0xF7, 0xE0, 0xE5, 0x02, 0x00, 0x94, 0xA6, 0xD7, 0xD0, 0x0A, 0x00, 0x00, 0x00]);
        //let mut mem_writer = MemWriter::new();
        let gzip_reader_res = GZipReader::new(comp_reader, 1);
        let mut gzip_reader = gzip_reader_res.unwrap();
        let mut out_buf = [0u8, ..64];
        let out_len = gzip_reader.read(out_buf);
        let decomp_buf = out_buf.slice(0, out_len.unwrap());
        println(fmt!("gzip_reader.read(): %?", decomp_buf));
    }

}

