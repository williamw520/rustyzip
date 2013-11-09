// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0.  If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
// 
// Software distributed under the License is distributed on an "AS IS" basis,
// WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for 
// the specific language governing rights and limitations under the License.
//
// The Original Code is: zip.rs
// The Initial Developer of the Original Code is: William Wong (williamw520@gmail.com)
// Portions created by William Wong are Copyright (C) 2013 William Wong, All Rights Reserved.


/*!

The zip module supports working with zip file and the file items within it.

*/


use std::str;
use std::num;
use std::vec;
use std::iter::{Iterator};
use std::rt::io::{Reader, Writer, Decorator};
use std::rt::io::{io_error, IoError};
use std::rt::io::{SeekSet, SeekEnd};
use std::rt::io::fs::File;



static CD_METADATA_MAGIC: u32   = 0x06054B50u32;
static CD_HEADER_MAGIC: u32     = 0x02014B50u32;
static LOCAL_HEADEr_MAGIC: u32  = 0x04034B50u32;
static LOCAL_DESC_MAGIC: u32    = 0x08074B50u32;

// #define VERSION_MADE            0xB17       // 0xB00 is win32 os-code. 0x17 is 23 in decimal: zip 2.3
// #define VERSION_NEEDED          20          // Needs PKUNZIP 2.0 to unzip it

// internal file attribute
// #define UNKNOWN (-1)
// #define BINARY  0
// #define ASCII   1

// #define BEST -1                 // Use best method (deflation or store)
// #define STORE 0                 // Store method
// #define DEFLATE 8               // Deflation method

static CD_METADATA_SIZE: uint       = 22u;      // including 2 bytes comment size.
static MAX_COMMENT_SIZE: uint       = 0xFFFFu;
static MAX_CD_METADATA_SEARCH: uint = CD_METADATA_SIZE + MAX_COMMENT_SIZE;
static CD_FILE_HEADER_SIZE: uint    = 46u;      // leading size for central directory header, before variable size fields.
static LOCAL_FILE_HEADER_SIZE: uint = 30u;      // leading size for local header, before variable size fields.




/// ZipFile structure to operate on a zip file.
pub struct ZipFile {
    /// Zip file's metadata for central directories.
    cd_metadata:        CDMetaData,
    priv inner_file:    File,
}


impl ZipFile {

    /// Opens a zip file for reading its meta data or its file items.
    pub fn open(file: File) -> Result<ZipFile, ~str> {
        let mut zip_file = ZipFile {
            cd_metadata:    CDMetaData::new(),
            inner_file:     file,
        };
        match zip_file.cd_metadata.read_cd_metadata(&mut zip_file.inner_file) {
            Ok(_)   => Ok(zip_file),
            Err(s)  => Err(s)
        }
    }

    /// Return an iterator ready to read each ZipEntry from the zip file.
    pub fn zip_entry_iter<'a>(&'a mut self) -> ZipEntry32Iterator<'a> {
        // Seek to file position at the beginning of cd directories.
        self.inner_file.seek(self.cd_metadata.cd_entry_begin_offset as i64, SeekSet);
        // Let the iterator to read each entry one at a time.
        ZipEntry32Iterator {
            zip_file:   self,
            index:      0u16,
            file_pos:   0u64,
            finished:   false,
        }
    }

    /// Return the list of all ZipEntries of the zip file.
    pub fn get_zip_entries(&mut self) -> Result<~[ZipEntry32], ~str> {
        // Seek to file position at the beginning of cd directories.
        self.inner_file.seek(self.cd_metadata.cd_entry_begin_offset as i64, SeekSet);
        // Read all the entries in one shot.
        let buf = read_upto(&mut self.inner_file, self.cd_metadata.cd_size as uint);
        if buf.len() != self.cd_metadata.cd_size as uint {
            return Err(format!("Fail to read all the zip entries.  Only read {:u} bytes out of {:u} total bytes.", buf.len(), self.cd_metadata.cd_size));
        }

        let mut entries = ~[];
        let mut offset = 0;
        for _ in range(0, self.cd_metadata.cd_entry_count) {
            let mut entry = ZipEntry32::new();
            match entry.unpack_zip_entry(buf, offset) {
                Ok(offset2) => {
                    offset = entry.unpack_zip_entry_extra(buf, offset2);
                },
                Err(s) => return Err(s)
            }
            entries.push(entry);
        }
        Ok(entries)
    }

    fn zip_entry_reader<'a>(&'a mut self, entry: &ZipEntry32) -> ZipReader<'a> {
        self.inner_file.seek(entry.local_header_offset as i64, SeekSet);
        ZipReader {
            zip_file:   self,
            zip_entry:  entry.clone(),
            is_eof:     false,
        }
    }

}


/// A zip file's central directory metadata, located at the end of the file.
pub struct CDMetaData {
    /// number of this disk
    disk_number:            u16,
    /// number of the disk with the start of the central directory
    cd_disk_number:         u16,
    /// total number of entries in the central directory on this disk
    cd_entry_count_on_disk: u16,
    /// total number of entries in the central directory
    cd_entry_count:         u16,
    /// size of the central directory
    cd_size:                u32,
    /// offset of start of central directory
    cd_entry_begin_offset:  u32,
    /// ZIP file comment length
    comment_length:         u16,
    /// file comment
    comment:                Option<~str>,

    // Additional info

    /// size of the zip file
    file_size:              u64,
}

impl CDMetaData {

    fn new() -> CDMetaData {
        CDMetaData {
            disk_number:            0u16,
            cd_disk_number:         0u16,
            cd_entry_count_on_disk: 0u16,
            cd_entry_count:         0u16,
            cd_size:                0u32,
            cd_entry_begin_offset:  0u32,
            comment_length:         0u16,
            comment:                None,
            file_size:              0u64,
        }
    }

    fn read_cd_metadata(&mut self, file: &mut File) -> Result<uint, ~str> {
        // Go to the end of the file and start searching for central directory metadata
        file.seek(0i64, SeekEnd);
        self.file_size = file.tell();
        if self.file_size < CD_METADATA_SIZE as u64{
            return Err(~"File too small to be a valid zip file.");
        }

        let max_search_size = num::min(self.file_size, MAX_CD_METADATA_SEARCH as u64) as uint;
        file.seek(-(max_search_size as i64), SeekEnd);
        let mut buf = vec::from_elem(max_search_size, 0u8);
        let read_len = read_buf_upto(file, buf, 0, max_search_size);

        for i in range(0, read_len - 4) {
            // The central directory metadata signature is 0x06054b50
            if buf[i] == 0x50u8 && buf[i+1] == 0x4bu8 && buf[i+2] == 0x05u8 && buf[i+3] == 0x06u8 {
                let mut offset = i;

                offset += 4;
                self.disk_number = unpack_u16_le(buf, offset);
                offset += 2;
                self.cd_disk_number = unpack_u16_le(buf, offset);
                offset += 2;
                self.cd_entry_count_on_disk = unpack_u16_le(buf, offset);
                offset += 2;
                self.cd_entry_count = unpack_u16_le(buf, offset);
                offset += 2;
                self.cd_size = unpack_u32_le(buf, offset);
                offset += 4;
                self.cd_entry_begin_offset = unpack_u32_le(buf, offset);
                offset += 4;
                self.comment_length = unpack_u16_le(buf, offset);
                offset += 2;
                if self.comment_length > 0 {
                    self.comment = Some(str::from_utf8(buf.slice(offset, offset + self.comment_length as uint)));
                }

                println(format!("{:?}", self));

                return Ok(0);
            }
        }
        Err(~"Zip file central directory signature missing.")
    }

}


/// A file item entry for a file item embedded in a zip file.
#[deriving(Clone)]
pub struct ZipEntry32 {
    /// version of zip format created this entry
    version_made_by:            u16,
    /// version needed to extract
    version_needed:             u16,
    ///general purpose bit flag
    general_flag:               u16,
    /// compression method
    compression_method:         u16,
    /// last mod file time
    modified_time:              u16,
    /// last mod file date
    modified_date:              u16,
    /// crc-32
    crc32:                      u32,
    /// compressed size
    compressed_size:            u32,
    /// uncompressed size
    uncompressed_size:          u32,
    /// file name length
    file_name_length:           u16,
    /// extra field length
    extra_field_length:         u16,
    /// file comment length
    file_comment_length:        u16,
    /// disk number start
    disk_number_start:          u16,
    /// internal file attributes
    internal_file_attributes:   u16,
    /// external file attributes
    external_file_attributes:   u32,
    /// relative offset of local header
    local_header_offset:        u32,
    /// file name
    file_name:                  Option<~str>,
    /// extra field
    extra_field:                Option<~str>,
    /// file comment
    file_comment:               Option<~str>,
}

impl ZipEntry32 {

    fn new() -> ZipEntry32 {
        ZipEntry32 {
            version_made_by:            0u16,
            version_needed:             0u16,
            general_flag:               0u16,
            compression_method:         0u16,
            modified_time:              0u16,
            modified_date:              0u16,
            crc32:                      0u32,
            compressed_size:            0u32,
            uncompressed_size:          0u32,
            file_name_length:           0u16,
            extra_field_length:         0u16,
            file_comment_length:        0u16,
            disk_number_start:          0u16,
            internal_file_attributes:   0u16,
            external_file_attributes:   0u32,
            local_header_offset:        0u32,
            file_name:                  None,
            extra_field:                None,
            file_comment:               None,
        }
    }

    // Unpack the fixed header of the zip entry.
    fn unpack_zip_entry(&mut self, buf: &[u8], mut offset: uint) -> Result<uint, ~str> {

        if unpack_u32_le(buf, offset) != CD_HEADER_MAGIC {
            return Err(~"Zip file entry signature mismatched.");
        }
        offset += 4;

        self.version_made_by = unpack_u16_le(buf, offset);
        offset += 2;
        self.version_needed = unpack_u16_le(buf, offset);
        offset += 2;
        self.general_flag = unpack_u16_le(buf, offset);
        offset += 2;
        self.compression_method = unpack_u16_le(buf, offset);
        offset += 2;
        self.modified_time = unpack_u16_le(buf, offset);
        offset += 2;
        self.modified_date = unpack_u16_le(buf, offset);
        offset += 2;
        self.crc32 = unpack_u32_le(buf, offset);
        offset += 4;
        self.compressed_size = unpack_u32_le(buf, offset);
        offset += 4;
        self.uncompressed_size = unpack_u32_le(buf, offset);
        offset += 4;
        self.file_name_length = unpack_u16_le(buf, offset);
        offset += 2;
        self.extra_field_length = unpack_u16_le(buf, offset);
        offset += 2;
        self.file_comment_length = unpack_u16_le(buf, offset);
        offset += 2;
        self.disk_number_start = unpack_u16_le(buf, offset);
        offset += 2;
        self.internal_file_attributes = unpack_u16_le(buf, offset);
        offset += 2;
        self.external_file_attributes = unpack_u32_le(buf, offset);
        offset += 4;
        self.local_header_offset = unpack_u32_le(buf, offset);
        offset += 4;

        return Ok(offset);
    }

    fn get_extra_length(&self) -> uint {
        return self.file_name_length as uint + self.extra_field_length as uint + self.file_comment_length as uint;
    }

    // Unpack the variable length header of the zip entry.
    fn unpack_zip_entry_extra(&mut self, buf: &[u8], mut offset: uint) -> uint {
        if self.file_name_length > 0 {
            self.file_name = Some(str::from_utf8( buf.slice(offset, offset + self.file_name_length as uint) ));
            offset += self.file_name_length as uint;
        }
        if self.extra_field_length > 0 {
            self.extra_field = Some(str::from_utf8( buf.slice(offset, offset + self.extra_field_length as uint) ));
            offset += self.extra_field_length as uint;
        }
        if self.file_comment_length > 0 {
            self.file_comment = Some(str::from_utf8( buf.slice(offset, offset + self.file_comment_length as uint) ));
            offset += self.file_comment_length as uint;
        }
        offset
    }

    fn read_zip_entry(file: &mut File) -> Result<ZipEntry32, ~str> {
        let mut buf = [0u8, ..CD_FILE_HEADER_SIZE];
        let read_len = read_buf_upto(file, buf, 0, CD_FILE_HEADER_SIZE);
        if read_len < CD_FILE_HEADER_SIZE {
            return Err(~"Zip file entry does not have enough data.");
        }
        
        let mut entry = ZipEntry32::new();
        match entry.unpack_zip_entry(buf, 0) {
            Err(s) => return Err(s),
            Ok(_) => {
                let buf = read_upto(file, entry.get_extra_length());
                entry.unpack_zip_entry_extra(buf, 0);
            }
        }
        Ok(entry)
    }

}

/// An iterator over the list of ZipEntry read from the zip file.
pub struct ZipEntry32Iterator<'self> {
    priv zip_file:  &'self mut ZipFile,
    priv index:     u16,
    priv file_pos:  u64,
    priv finished:  bool,
}


impl<'self> Iterator<ZipEntry32> for ZipEntry32Iterator<'self> {

    fn next(&mut self) -> Option<ZipEntry32> {
        if self.finished {
            return None;
        }
        if self.index >= self.zip_file.cd_metadata.cd_entry_count {
            self.finished = true;
            return None;
        }

        self.index += 1;
        self.finished = (self.index == self.zip_file.cd_metadata.cd_entry_count);

        match ZipEntry32::read_zip_entry(&mut self.zip_file.inner_file) {
            Ok(entry) => Some(entry),
            Err(s) => fail!(s)      // TODO: return error
        }
    }


    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        if self.finished {
            (0u, Some(0u))
        } else {
            (self.index as uint, Some(self.zip_file.cd_metadata.cd_entry_count as uint))
        }
    }
}

/// Reader for reading the content of the file item at the zip entry.
pub struct ZipReader<'self> {
    priv zip_file:  &'self mut ZipFile,
    priv zip_entry: ZipEntry32,
    priv is_eof:    bool,
}

impl<'self> Reader for ZipReader<'self> {
    /// Read the decompressed data from the file item inside the zip file.
    fn read(&mut self, output_buf: &mut [u8]) -> Option<uint> {

        Some(0)
    }

    fn eof(&mut self) -> bool {
        return self.is_eof;
    }
}



/// Pack a u16 into byte buffer in little-endian
fn pack_u16_le(buf: &mut [u8], offset: uint, value: u16) -> uint {
    buf[offset + 0] = (value >> 0) as u8;
    buf[offset + 1] = (value >> 8) as u8;
    offset + 2
}

/// Unpack a u16 from byte buffer in little-endian
fn unpack_u16_le(buf: &[u8], offset: uint) -> u16 {
    ( ((buf[offset + 0] as u16) & 0xFF)      ) |
    ( ((buf[offset + 1] as u16) & 0xFF) << 8 )
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

/// Pack a string into a zero-terminated buffer.
fn to_strz(str_value: &str) -> ~[u8] {
    let str_bytes = str_value.as_bytes();
    let mut buf = vec::from_elem(str_bytes.len() + 1, 0u8);
    vec::bytes::copy_memory(buf, str_bytes, str_bytes.len());
    buf[buf.len() - 1] = 0;
    return buf;
}

/// Read a zero-terminated str.  Read until encountering the terminating 0.
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

fn read_upto<R: Reader>(reader: &mut R, len_to_read: uint) -> ~[u8] {
    let mut buf = vec::from_elem(len_to_read, 0u8);
    read_buf_upto(reader, buf, 0, len_to_read);
    return buf;
}

/// Read data upto the len_to_read, unless encounters EOF.
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


#[cfg(test)]
mod tests {

    use std::rt::io::Reader;
    use std::rt::io::mem::MemReader;
    use std::rt::io::mem::MemWriter;


}

