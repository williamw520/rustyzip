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
use std::num;
use std::vec;

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
static FTEXT: u8    = 1;        // File is text file
static FHCRC: u8    = 2;        // Header CRC
static FEXTRA: u8   = 4;        // Extra field
static FNAME: u8    = 8;        // File name
static FCOMMENT: u8 = 16;       // File comment

static END_LENGTH: uint = 8;    // length of end section of a gzip file - 4 bytes CRC, 4 bytes original size


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
    cmp_crc32:     u32,
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
            cmp_crc32:     0,
        }
    }

    fn readHeader<R: Reader>(&mut self, reader: &mut BitReader<R>) -> Result<uint, ~str> {

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

    fn readHeaderExtra<R: Reader>(&mut self, reader: &mut BitReader<R>) -> Result<uint, ~str> {

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

    fn readEndSection(&mut self, end_buf: &[u8], end_len: uint) -> Result<uint, ~str> {
        if end_len < END_LENGTH {
            return Err(fmt!("Not enough data in gzip end section.  Bytes missing: %?", (END_LENGTH - end_len)));
        }

        self.crc32 = ioutil::unpack_u32_le(end_buf, 0);
        self.original_size = ioutil::unpack_u32_le(end_buf, 4);

        println(fmt!("    crc32: %?", self.crc32));
        println(fmt!("cmp_crc32: %?", self.cmp_crc32));
        println(fmt!("original_size: %?", self.original_size));

        Ok(0)
    }

    fn readCompressedData<R: Reader>(&mut self, reader: &mut BitReader<R>) -> Result<uint, ~str> {
        let mut decomp_data : ~[u8] = ~[];
        let mut end_buf = [0u8, ..END_LENGTH];
        let mut end_len = 0u;

        let mut decomp = Decompressor::new();
        let status = decomp.decompress_upcalls(
            // upcall function to read input data for decompression
            |in_buf| {
                if reader.eof() {
                    0                           // EOF
                } else {
                    // Test small read size
                    // let mut sbuf = [0u8, ..4];
                    // match reader.read(sbuf) {
                    //     Some(nread) => {
                    //         vec::bytes::copy_memory(in_buf, sbuf, nread);
                    //         nread
                    //     },
                    //     None =>
                    //         0
                    // }
                    match reader.read(in_buf) {
                        Some(nread) => nread,   // read number of bytes read, including 0 for EOF
                        None => 0               // EOF
                    }
                }
            },
            // upcall function to write the decompressed data
            |out_buf, _ /* is_eof */| {
                self.cmp_crc32 = update_crc(self.cmp_crc32, out_buf, 0, out_buf.len());
                decomp_data.push_all(out_buf);
                false                           // don't abort
            },
            // upcall function to handle the remaining input data that are not part of the compressed data.
            |rest_buf| {
                end_len = rest_buf.len();
                vec::bytes::copy_memory(end_buf, rest_buf, num::min(END_LENGTH, end_len));
                let remaining_in_rest_buf = rest_buf.slice_from(num::min(END_LENGTH, end_len));
                if end_len < END_LENGTH {
                    end_len += reader.read_buf_upto(end_buf, end_len, END_LENGTH - end_len);
                }
            } );
        decomp.free();

        println(fmt!("decomp_data: %?", decomp_data));

        match status {
            INFLATE_STATUS_DONE => 
                self.readEndSection(end_buf, end_len),
            _ => 
                Err(fmt!("Failed to decompress data.  Status: %?", status))
        }
    }

    /// Decompress the data stream
    pub fn decompress<R: Reader>(&mut self, reader: &mut BitReader<R>) -> Result<uint, ~str> {
        match self.readHeader(reader) {
            Ok(_) => {
                match self.readHeaderExtra(reader) {
                    Ok(_) =>
                        self.readCompressedData(reader),
                    Err(s) => 
                        Err(s)
                }
            },
            Err(s) => 
                Err(s)
        }
    }

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

    use super::*;

    #[test]
    fn test_generate_crc_table() {
        // Uncomment to generate the crc table text.
        //generate_crc_table();
    }

}

