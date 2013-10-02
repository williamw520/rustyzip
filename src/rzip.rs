/******************************************************************************
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.  If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/.
 * 
 * Software distributed under the License is distributed on an "AS IS" basis, 
 * WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for 
 * the specific language governing rights and limitations under the License.
 *
 * The Original Code is: rzip.rs
 * The Initial Developer of the Original Code is: William Wong (williamw520@gmail.com)
 * Portions created by William Wong are Copyright (C) 2013 William Wong, All Rights Reserved.
 *
 ******************************************************************************/

// export RUST_LOG=rzip,rustyzip


extern mod extra;
extern mod rustyzip;

use std::os;
use std::result::*;
use extra::getopts::*;

use std::rt::io::mem::MemReader;

use common::apputil;
//use common::strutil;
//use common::netutil;
//use rustyzip::rustyzip_lib::gzip;
use rustyzip::rustyzip_lib::gzip::GZip;
//use rustyzip::rustyzip_lib::deflate;
//use rustyzip::rustyzip_lib::deflate::Inflater;
use rustyzip::common::bitstream::BitReader;


mod common {
    pub mod apputil;
    pub mod strutil;
}




// Constants
static DEFAULT_PORT : int           = 11211;    // memcached default port
static DEFAULT_MEMORY_SIZE : int    = 128;      // memory size in megabyte


struct Options {
    help:           bool,
    decompress:     bool,
}

impl Options {

    pub fn new() -> Options {
        return Options {
            help: false,
            decompress: false,
        };
    }

    pub fn from_args(args: &~[~str]) -> Options {
        let mut options = Options::new();
        options.parse_arguments(args);
        return options;
    }

    fn parse_arguments(&mut self, args: &~[~str]) {
        let opts = ~[
                     optflag("h"),
                     optflag("help"),
                     optflag("d"),
                     optflag("decompress"),
                     ];
        let matches = match getopts((*args).tail(), opts) {
            Ok(m)   => { m }
            Err(f)  => { fail!(fmt!("%?", f)); }
        };

        self.help = matches.opt_present("h") || matches.opt_present("help");
        self.decompress = matches.opt_present("d") || matches.opt_present("decompress");
    }

}



fn print_usage(args: &~[~str]) {
    let program = apputil::get_program(args);
    println(fmt!("Usage: %s -d --decompress -h --help", program));
}



fn main()  {

    debug!("main() enter");

    let args = os::args();
    let options = Options::from_args(&args);
    debug!( fmt!("options = %?", options) );

    if options.help {
        print_usage(&args);
    } else {
        if options.decompress {
            decompress();
        } else {
            compress();
        }
    }

    debug!("main() exit");

}


fn decompress() {
    debug!( "decompress()" );

    // gzip'ed file data for testing.
    let reader_gzip = MemReader::new(~[0x1f, 0x8B, 0x08, 0x08, 0x54, 0x3C, 0x3D, 0x52, 0x00, 0x03, 0x74, 0x65, 0x73, 0x74, 0x31, 0x00, 0x73, 0x74, 0x72, 0x76, 0x71, 0x75, 0x73, 0xF7, 0xE0, 0xE5, 0x02, 0x00, 0x94, 0xA6, 0xD7, 0xD0, 0x0A, 0x00, 0x00, 0x00]);
    let mut bitreader = BitReader::new(reader_gzip, true);
    let mut gzip = GZip::new();
    let result = gzip.decompress(&mut bitreader);

    println(fmt!("%?", result));

    // // deflated file data for testing.
    // let reader_defl = MemReader::new(~[0xCB, 0x48, 0xCD, 0xC9, 0xC9, 0xE7, 0x02, 0x00]);   // hello;
    // let bitreader = BitReader::new(reader_defl, true);
    // let mut inflater = Inflater::new();
    // inflater.inflate(bitreader);

    //let reader = stdio::stdin();
    //debug!( fmt!("%?", reader) );
    //let bytes = io::stdin().read_to_end();
    // let bufReader = @BufReader::new(bytes);
    // let mut decoder = new(bufReader as @Reader);
    // if(decoder.inflate()) {
    //     print(fmt!("Success: %u bytes", decoder.writeBuf.len()));
    // } else {
    //     print("Failed");
    // }
}


fn compress() {
    debug!( "compress()" );

}
