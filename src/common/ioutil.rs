/******************************************************************************
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.  If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/.
 * 
 * Software distributed under the License is distributed on an "AS IS" basis, 
 * WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for 
 * the specific language governing rights and limitations under the License.
 *
 * The Original Code is: ioutil.rs
 * The Initial Developer of the Original Code is: William Wong (williamw520@gmail.com)
 * Portions created by William Wong are Copyright (C) 2013 William Wong, All Rights Reserved.
 *
 ******************************************************************************/



use std::str;
use std::vec;
use std::rt::io::Reader;
use std::rt::io::ReaderUtil;


/// Platform independent, language independent way of packing data into byte buffer


/// Pack a u8 into byte buffer
pub fn pack_u8(buf: &mut [u8], offset: uint, value: u8) -> uint {
    buf[offset] = value;
    offset + 1
}

/// Unpack a u8 from byte buffer
pub fn unpack_u8(buf: &[u8], offset: uint) -> u8 {
    buf[offset]
}

/// Pack a u16 into byte buffer in big-endian (network order)
pub fn pack_u16_be(buf: &mut [u8], offset: uint, value: u16) -> uint {
    buf[offset + 0] = (value >> 8) as u8;
    buf[offset + 1] = (value >> 0) as u8;
    offset + 2
}

/// Unpack a u16 from byte buffer in big-endian (network order)
pub fn unpack_u16_be(buf: &[u8], offset: uint) -> u16 {
    ( ((buf[offset + 0] as u16) & 0xFF) << 8 ) |
    ( ((buf[offset + 1] as u16) & 0xFF)      )
}

/// Pack a u32 into byte buffer in big-endian (network order)
pub fn pack_u32_be(buf: &mut [u8], offset: uint, value: u32) -> uint {
    buf[offset + 0] = (value >> 24) as u8;
    buf[offset + 1] = (value >> 16) as u8;
    buf[offset + 2] = (value >> 8) as u8;
    buf[offset + 3] = (value >> 0) as u8;
    offset + 4
}

/// Unpack a u32 from byte buffer in big-endian (network order)
pub fn unpack_u32_be(buf: &[u8], offset: uint) -> u32 {
    ( ((buf[offset + 0] as u32) & 0xFF) << 24 ) |
    ( ((buf[offset + 1] as u32) & 0xFF) << 16 ) |
    ( ((buf[offset + 2] as u32) & 0xFF) << 8  ) |
    ( ((buf[offset + 3] as u32) & 0xFF)       )
}

/// Pack a u64 into byte buffer in big-endian (network order)
pub fn pack_u64_be(buf: &mut [u8], offset: uint, value: u64) -> uint {
    buf[offset + 0] = (value >> 56) as u8;
    buf[offset + 1] = (value >> 48) as u8;
    buf[offset + 2] = (value >> 40) as u8;
    buf[offset + 3] = (value >> 32) as u8;
    buf[offset + 4] = (value >> 24) as u8;
    buf[offset + 5] = (value >> 16) as u8;
    buf[offset + 6] = (value >> 8) as u8;
    buf[offset + 7] = (value >> 0) as u8;
    offset + 8
}

/// Unpack a u64 from byte buffer in big-endian (network order)
pub fn unpack_u64_be(buf: &[u8], offset: uint) -> u64 {
    ( ((buf[offset + 0] as u64) & 0xFF) << 56 ) |
    ( ((buf[offset + 1] as u64) & 0xFF) << 48 ) |
    ( ((buf[offset + 2] as u64) & 0xFF) << 40 ) |
    ( ((buf[offset + 3] as u64) & 0xFF) << 32 ) |
    ( ((buf[offset + 4] as u64) & 0xFF) << 24 ) |
    ( ((buf[offset + 5] as u64) & 0xFF) << 16 ) |
    ( ((buf[offset + 6] as u64) & 0xFF) << 8  ) |
    ( ((buf[offset + 7] as u64) & 0xFF)       )
}


/// Pack a u16 into byte buffer in little-endian
pub fn pack_u16_le(buf: &mut [u8], offset: uint, value: u16) -> uint {
    buf[offset + 0] = (value >> 0) as u8;
    buf[offset + 1] = (value >> 8) as u8;
    offset + 2
}

/// Unpack a u16 from byte buffer in little-endian
pub fn unpack_u16_le(buf: &[u8], offset: uint) -> u16 {
    ( ((buf[offset + 0] as u16) & 0xFF)      ) |
    ( ((buf[offset + 1] as u16) & 0xFF) << 8 )
}

/// Pack a u32 into byte buffer in little-endian
pub fn pack_u32_le(buf: &mut [u8], offset: uint, value: u32) -> uint {
    buf[offset + 0] = (value >> 0) as u8;
    buf[offset + 1] = (value >> 8) as u8;
    buf[offset + 2] = (value >> 16) as u8;
    buf[offset + 3] = (value >> 24) as u8;
    offset + 4
}

/// Unpack a u32 from byte buffer in little-endian
pub fn unpack_u32_le(buf: &[u8], offset: uint) -> u32 {
    ( ((buf[offset + 0] as u32) & 0xFF)       ) |
    ( ((buf[offset + 1] as u32) & 0xFF) << 8  ) |
    ( ((buf[offset + 2] as u32) & 0xFF) << 16 ) |
    ( ((buf[offset + 3] as u32) & 0xFF) << 24 )
}

/// Pack a u64 into byte buffer in little-endian
pub fn pack_u64_le(buf: &mut [u8], offset: uint, value: u64) -> uint {
    buf[offset + 0] = (value >> 0) as u8;
    buf[offset + 1] = (value >> 8) as u8;
    buf[offset + 2] = (value >> 16) as u8;
    buf[offset + 3] = (value >> 24) as u8;
    buf[offset + 4] = (value >> 32) as u8;
    buf[offset + 5] = (value >> 40) as u8;
    buf[offset + 6] = (value >> 48) as u8;
    buf[offset + 7] = (value >> 56) as u8;
    offset + 8
}

/// Unpack a u64 from byte buffer in little-endian
pub fn unpack_u64_le(buf: &[u8], offset: uint) -> u64 {
    ( ((buf[offset + 0] as u64) & 0xFF)       ) |
    ( ((buf[offset + 1] as u64) & 0xFF) << 8  ) |
    ( ((buf[offset + 2] as u64) & 0xFF) << 16 ) |
    ( ((buf[offset + 3] as u64) & 0xFF) << 24 ) |
    ( ((buf[offset + 4] as u64) & 0xFF) << 32 ) |
    ( ((buf[offset + 5] as u64) & 0xFF) << 40 ) |
    ( ((buf[offset + 6] as u64) & 0xFF) << 48 ) |
    ( ((buf[offset + 7] as u64) & 0xFF) << 56 )
}


pub fn pack_str(buf: &mut [u8], offset: uint, str_value: &str) -> uint {
    let str_bytes = str_value.as_bytes();
    return copy_bytes(buf, offset, str_bytes, 0, str_bytes.len());
}

pub fn to_strz(str_value: &str) -> ~[u8] {
    let str_bytes = str_value.as_bytes();
    let mut buf = vec::from_elem(str_bytes.len() + 1, 0u8);
    vec::bytes::copy_memory(buf, str_bytes, str_bytes.len());
    buf[buf.len() - 1] = 0;
    return buf;
}


pub fn copy_bytes(to_buf: &mut [u8],  to_offset: uint,  from_buf: &[u8],  from_offset: uint,  len: uint) -> uint {
    let to_slice = to_buf.mut_slice(to_offset, to_offset + len);
    let from_slice = from_buf.slice(from_offset, from_offset + len);
    vec::bytes::copy_memory(to_slice, from_slice, len);
    to_offset + len
}


pub fn trunc_bytes(bytes: &[u8]) -> u32 {
    // Take the first 4 bytes as int
    return unpack_u32_be(bytes, 0);
}


// Add the following functions to any Reader implementation
pub trait ReaderEx {
    fn read_upto(&mut self, len_to_read: uint) -> ~[u8];
    fn read_buf_upto(&mut self, buf: &mut [u8], offset: uint, len_to_read: uint) -> uint;
    fn read_strz(&mut self) -> ~str;
    fn read_u16_le(&mut self) -> u16;
    fn read_u16_be(&mut self) -> u16;
    fn read_u32_le(&mut self) -> u32;
    fn read_u32_be(&mut self) -> u32;
    fn read_u64_le(&mut self) -> u64;
    fn read_u64_be(&mut self) -> u64;
}

impl<R: Reader> ReaderEx for R {

    fn read_upto(&mut self, len_to_read: uint) -> ~[u8] {
        let mut buf = vec::from_elem(len_to_read, 0u8);
        self.read_buf_upto(buf, 0, len_to_read);
        return buf;
    }

    fn read_buf_upto(&mut self, buf: &mut [u8], offset: uint, len_to_read: uint) -> uint {
        let mut total_read = 0u;
        while total_read < len_to_read {
            let remaining_len = len_to_read - total_read;
            let begin = offset + total_read;
            let end   = offset + total_read + remaining_len;
            let slice_buf = buf.mut_slice(begin, end);
            match self.read(slice_buf) {
                Some(read_len) => total_read = total_read + read_len,
                None => break
            }
        }
        return total_read;
    }

    // Read a zero-terminated str.  Read until encountering the terminating 0.
    fn read_strz(&mut self) -> ~str {
        let mut buf = ~[];
        loop {
            match self.read_byte() {
                Some(0)     => break,
                Some(ch)    => buf.push(ch),
                None        => break
            }
        }
        return str::from_utf8(buf);
    }

    fn read_u16_le(&mut self) -> u16 {
        let mut buf = [0, 2];
        self.read_buf_upto(buf, 0, 2);
        return unpack_u16_le(buf, 0);
    }

    fn read_u16_be(&mut self) -> u16 {
        let mut buf = [0, 2];
        self.read_buf_upto(buf, 0, 2);
        return unpack_u16_be(buf, 0);
    }

    fn read_u32_le(&mut self) -> u32 {
        let mut buf = [0, 4];
        self.read_buf_upto(buf, 0, 4);
        return unpack_u32_le(buf, 0);
    }

    fn read_u32_be(&mut self) -> u32 {
        let mut buf = [0, 4];
        self.read_buf_upto(buf, 0, 4);
        return unpack_u32_be(buf, 0);
    }

    fn read_u64_le(&mut self) -> u64 {
        let mut buf = [0, 4];
        self.read_buf_upto(buf, 0, 4);
        return unpack_u64_le(buf, 0);
    }

    fn read_u64_be(&mut self) -> u64 {
        let mut buf = [0, 4];
        self.read_buf_upto(buf, 0, 4);
        return unpack_u64_be(buf, 0);
    }

}





#[test]
fn test() {
    let mut buf = vec::from_elem(16, 0u8);
    let mut offset;

    offset = pack_u8(buf, 0, 1);
    offset = pack_u8(buf, offset, 2);
    //println( fmt!("%? %?", buf, offset) );
    if offset != 2 { fail!() };
    if buf[0] != 1 { fail!() };
    if buf[1] != 2 { fail!() };

    offset = pack_u16_be(buf, 0, 0x0102);
    //println( fmt!("%? %?", buf, offset) );
    if offset != 2 { fail!() };
    if buf[0] != 1 { fail!() };
    if buf[1] != 2 { fail!() };

    offset = pack_str(buf, 0, "ABCD");
    //println( fmt!("%? %?", buf, offset) );
    if offset != 4 { fail!() };
    if buf[0] != 'A' as u8 { fail!() };
    if buf[1] != 'B' as u8 { fail!() };
    if buf[2] != 'C' as u8 { fail!() };
    if buf[3] != 'D' as u8 { fail!() };

    pack_u16_be(buf, 0, 12345);
    if unpack_u16_be(buf, 0) != 12345 { fail!() };

    pack_u32_be(buf, 0, 12345678);
    if unpack_u32_be(buf, 0) != 12345678 { fail!() };

    pack_u64_be(buf, 0, 12345678901234);
    if unpack_u64_be(buf, 0) != 12345678901234 { fail!() };

}

