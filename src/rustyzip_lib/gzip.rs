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


use std::rt::io::Reader;
//use std::rt::io::Writer;

use common::ioutil::ReaderEx;

use common::ioutil;
use common::bitstream::BitReader;
use super::deflate::INFLATE_STATUS_DONE;
use super::deflate::Decompressor;




static HEADER_FIXED_LEN: uint = 10;
static MAGIC1: u8 = 0x1f;
static MAGIC2: u8 = 0x8b;
static COMPRESSION_DEFLATE: u8 = 8;

// Header flags
static FTEXT: u8    = 1;	// File is text file
static FHCRC: u8    = 2;	// Header CRC
static FEXTRA: u8   = 4;	// Extra field
static FNAME: u8    = 8;	// File name
static FCOMMENT: u8 = 16;	// File comment



/// GZip to decompress a data stream
pub struct GZip {
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
    crc32:          u32,
    original_size:  u32,
}

impl GZip {

    pub fn new() -> GZip {
        GZip {
            id1:            0,
            id2:            0,
            compression:    0,
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
        }
    }

    fn readFixedHeader<R: Reader>(&mut self, reader: &mut BitReader<R>) -> Result<uint, ~str> {

        let mut buf = [0, ..HEADER_FIXED_LEN];
        if reader.read_buf_upto(buf, 0, HEADER_FIXED_LEN) != HEADER_FIXED_LEN {
            return Err(~"Too few data to be a valid gzip format.");
        }

        self.id1 = buf[0];
        self.id2 = buf[1];
        self.compression = buf[2];
        self.flags = buf[3];
        self.mtime = ioutil::unpack_u32_le(buf, 4);
        self.xflags = buf[8];
        self.os = buf[9];

        if self.id1 != MAGIC1 || self.id2 != MAGIC2 {
            return Err(~"Invalid gzip signature");
        }
        if self.compression != COMPRESSION_DEFLATE {
            return Err(~"Unsupported compression method");
        }

        Ok(0)
    }

    fn readExtraHeader<R: Reader>(&mut self, reader: &mut BitReader<R>) -> Result<uint, ~str> {

	    if (self.flags & FEXTRA) == FEXTRA {
            self.xfield_len = Some(reader.read_u16_le());
            self.xfield = Some(reader.read_upto(self.xfield_len.unwrap() as uint));
        }

	    if (self.flags & FNAME) == FNAME {
            self.filename = Some(reader.read_strz());
        }

	    if (self.flags & FCOMMENT) == FCOMMENT {
            self.comment = Some(reader.read_strz());
        }

        if (self.flags & FHCRC) == FHCRC {
            self.header_crc = Some(reader.read_u16_le());
        }

        Ok(0)
    }

    fn readFooter<R: Reader>(&mut self, reader: &mut BitReader<R>) -> Result<uint, ~str> {
        //self.crc32 = reader.read_u32_le();
        //self.original_size = reader.read_u32_le();
        Ok(0)
    }

    fn readCompressedData<R: Reader>(&mut self, reader: &mut BitReader<R>) -> Result<uint, ~str> {
        let mut decomp_data : ~[u8] = ~[];
        let mut remaining_data : ~[u8] = ~[];

        let mut decomp = Decompressor::new();
        let status = decomp.decompress_upcalls(
            // upcall function to read input data for decompression
            |in_buf| {
                if reader.eof() {
                    0
                } else {
                    match reader.read(in_buf) {
                        Some(nread) => nread,   // EOF if it's 0
                            None => 0               // EOF
                    }
                }
            },
            // upcall function to write the decompressed data
            |out_buf, _ /* is_eof */| {
                decomp_data.push_all(out_buf);
                false                           // don't abort
            },
            // upcall function to handle the remaining input data that are not part of the compressed data.
            |rest_buf| {
                remaining_data.push_all(rest_buf);
            } );
        decomp.free();

        println(fmt!("decomp_data: %?", decomp_data));
        println(fmt!("remaining_data: %?", remaining_data));

        match status {
            INFLATE_STATUS_DONE => Ok(0),
            _ => Err(fmt!("Failed to decompress data.  Status: %?", status))
        }
    }

    /// Decompress the data stream
    pub fn decompress<R: Reader>(&mut self, reader: &mut BitReader<R>) -> Result<uint, ~str> {
        match self.readFixedHeader(reader) {
            Ok(_) => {
                match self.readExtraHeader(reader) {
                    Ok(_) => {
                        match self.readCompressedData(reader) {
                            Ok(_) => {
                                self.readFooter(reader)
                            },
                            Err(s) => Err(s)
                        }
                    },
                    Err(s) => Err(s)
                }
            },
            Err(s) => Err(s)
        }
    }

}


