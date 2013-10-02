/******************************************************************************
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.  If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/.
 * 
 * Software distributed under the License is distributed on an "AS IS" basis, 
 * WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for 
 * the specific language governing rights and limitations under the License.
 *
 * The Original Code is: deflate.rs
 * The Initial Developer of the Original Code is: William Wong (williamw520@gmail.com)
 * Portions created by William Wong are Copyright (C) 2013 William Wong, All Rights Reserved.
 *
 ******************************************************************************/


use std::rt::io::Reader;
//use std::rt::io::Writer;
use common::ioutil::ReaderEx;
use common::bitstream::BitReader;



use std::libc::{c_void, size_t, c_int};
use std::libc;
use std::vec;

pub mod rustrt {
    use std::libc::{c_int, c_void, size_t};

    #[link_name = "rustrt"]
    extern {
        pub fn tdefl_compress_mem_to_heap(psrc_buf: *c_void,
                                          src_buf_len: size_t,
                                          pout_len: *mut size_t,
                                          flags: c_int)
                                          -> *c_void;

        pub fn tinfl_decompress_mem_to_heap(psrc_buf: *c_void,
                                            src_buf_len: size_t,
                                            pout_len: *mut size_t,
                                            flags: c_int)
                                            -> *c_void;
    }
}




/// Inflater to decompress a data stream
pub struct Inflater {
    dummy: uint
}

impl Inflater {

    pub fn new() -> Inflater {
        Inflater {
            dummy: 0
        }
    }

    /// Decompress the data stream
    pub fn inflate<R: Reader>(&mut self, reader: &mut BitReader<R>) -> Result<uint, ~str> {
        loop {
            match (reader.read_bits(1), reader.read_bits(2)) {
                (Some(bfinal), Some(btype)) => {
                    match (btype) {
                        0 => {
                            debug!("Store");
                            reader.consume_buf_bits();
                            let len = reader.read_u16_le();
                            let nlen = reader.read_u16_le();
                            debug!(fmt!("nlen: %?", nlen));
                            let data = reader.read_upto(len as uint);
                            debug!(fmt!("data: %?", data));
                        },
                        1 => {
                            debug!("Fixed Huffman");
                        },
                        2 => {
                            debug!("Dynamic Huffman");
                        }
                        // Error
                        _ => return Err(~"Invalid encoding type in deflated stream.")
                    }

                    if bfinal == 1 {
                        break;
                    }
                },
                _ => return Err(~"Too few data in compressed data stream.")
            }
        }

        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("------"));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));
        println(fmt!("bit: %?", reader.read_bits(1)));

        Ok(0)
    }

}




#[test]
fn test_inflate() {
    println( fmt!("test_inflate") );

    let message = "it's alright. have a good time";
    let filename = &Path("./test/test1.gz");
    println( fmt!("**** filename: %?", filename) );

    // {
    //     let mut write_stream = file::open(filename, io::Create, io::ReadWrite).unwrap();
    //     write_stream.write(message.as_bytes());
    // }

    let mut read_stream = file::open(filename, io::Open, io::ReadWrite).unwrap();
    let mut read_buf = [0u8, .. 1028];
    let mut rs = read_stream.read(read_buf).unwrap();
    println( fmt!("**** rs: %?", rs) );

    // let read_str = match read_stream.read(read_buf).unwrap() {
    //     -1|0 => fail!("shouldn't happen"),
    //     n => str::from_utf8(read_buf.slice_to(n))
    // };
    
    // println( fmt!("**** test1.gz: %?", read_buf) );
    
}

