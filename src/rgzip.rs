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

// export RUST_LOG=rgzip,rustyzip


extern mod extra;
extern mod rustyzip;

use std::os;
use std::num;
use std::vec;
use std::result::{Result, Ok, Err};
use extra::getopts::{optflag, optopt, getopts};
use std::path::Path;
//use std::rt::io::stdio;
use std::rt::io::DEFAULT_BUF_SIZE;
use std::rt::io::file;
use std::rt::io::file::FileStream;
use std::rt::io::{Reader, Writer};
use std::rt::io::{Open, Create};
use std::rt::io::{Read, Write};
use std::rt::io::io_error;

use rustyzip::rustyzip_lib::gzip;
use rustyzip::rustyzip_lib::gzip::GZip;
use rustyzip::rustyzip_lib::gzip::GZipReader;
use rustyzip::rustyzip_lib::gzip::GZipWriter;

static VERSION_STR : &'static str = "0.8";
static MIN_SIZE_FACTOR : uint = 5;      // minimum size factor: 2^5 * 1K = 32K
static DEFAULT_SIZE_FACTOR : uint = 8;  // default size factor: 2^8 * 1K = 256K




enum Cmd {
    HELP, VERSION, COMPRESS, DECOMPRESS, LIST
}

struct Options {
    cmd:            Cmd,
    stdout:         bool,
    force:          bool,
    no_name:        bool,
    quiet:          bool,
    verbose:        bool,
    compress_level: uint,
    use_pipe:       bool,
    size_factor:    uint,
    files:          ~[~str],
}


impl Options {

    pub fn from_args(args: &~[~str]) -> Result<Options, ~str> {
        let mut options = Options {
            cmd: COMPRESS,          // default command is to compress
            stdout: false,
            force: false,
            no_name: false,
            quiet: false,
            verbose: false,
            compress_level: gzip::DEFAULT_COMPRESS_LEVEL,
            use_pipe: true,
            size_factor: DEFAULT_SIZE_FACTOR,
            files: ~[],
        };
        let opts = ~[
                     optflag("h"),
                     optflag("help"),
                     optflag("V"),
                     optflag("version"),
                     optflag("d"),
                     optflag("decompress"),
                     optflag("l"),
                     optflag("list"),
                     optflag("c"),
                     optflag("stdout"),
                     optflag("f"),
                     optflag("force"),
                     optflag("n"),
                     optflag("no-name"),
                     optflag("q"),
                     optflag("quiet"),
                     optflag("v"),
                     optflag("verbose"),
                     optflag("0"),
                     optflag("1"),
                     optflag("2"),
                     optflag("3"),
                     optflag("4"),
                     optflag("5"),
                     optflag("6"),
                     optflag("7"),
                     optflag("8"),
                     optflag("9"),
                     optflag("Pipe"),
                     optopt("b"),
                     optopt("bufsize"),
                     
                     ];

        match getopts((*args).tail(), opts) {
            Ok(matches) => {
                options.cmd = if matches.opt_present("h") || matches.opt_present("help") {  HELP } else { options.cmd };
                options.cmd = if matches.opt_present("V") || matches.opt_present("version") { VERSION } else { options.cmd };
                options.cmd = if matches.opt_present("d") || matches.opt_present("decompress") { DECOMPRESS } else { options.cmd };
                options.cmd = if matches.opt_present("l") || matches.opt_present("list") { LIST } else { options.cmd };

                options.stdout = matches.opt_present("c") || matches.opt_present("stdout");
                options.force = matches.opt_present("f") || matches.opt_present("force");
                options.no_name = matches.opt_present("n") || matches.opt_present("no-name");
                options.quiet = matches.opt_present("q") || matches.opt_present("quiet");
                options.verbose = matches.opt_present("v") || matches.opt_present("verbose");
                for level in range(0u, 10u) {
                    let slevel = fmt!("%u", level);
                    options.compress_level = if matches.opt_present(slevel) { level } else { options.compress_level };
                }
                options.use_pipe = !matches.opt_present("Pipe");
                let mut size_factor = if matches.opt_present("bufsize") { maybe_to_num(matches.opt_str("bufsize"), DEFAULT_SIZE_FACTOR) } else { DEFAULT_SIZE_FACTOR };
                size_factor = if matches.opt_present("b")       { maybe_to_num(matches.opt_str("b"), size_factor) } else { size_factor };
                options.size_factor = num::max(MIN_SIZE_FACTOR, size_factor);
                options.files = matches.free;

                debug!( fmt!("options = %?", options) );
                Ok(options)
            },
            Err(err) =>
                Err(err.to_err_msg())
        }
    }

}


fn maybe_to_num<T: FromStr>(s : Option<~str>, default_value : T) -> T {
    match s {
        Some(s) => to_num(s, default_value),
        None => default_value
    }
}

fn to_num<T: FromStr>(s : &str, default_value : T) -> T {
    match from_str::<T>(s.trim()) {
        Some(i) => i,
        None => default_value
    }
}

fn get_program(args: &~[~str]) -> ~str {
    let path: Path = GenericPath::from_str((*args)[0]);
    match path.filestem() {
        Some(name) => { name.to_owned() },
        None => ~""
    }
}

fn print_usage(args: &~[~str]) {
    println(fmt!("Usage: %s  -h --help -d --decompress -c --stdout FILE ...", get_program(args)));
}

fn print_version(args: &~[~str]) {
    println(fmt!("%s %s", get_program(args), VERSION_STR));
    println(fmt!("Written by William Wong"));
}


fn open_compressed_writer(options: &Options, filepath: &Path) -> Result<FileStream, ~str> {
    if options.stdout {
        //let writer = stdio::stdout();
        //return writer;
        fail2!("std::rt::io::stdout is not implemented yet");
    }

    let gz_filepath = filepath.to_str() + ".gz";
    let out_filepath = Path(gz_filepath);
    if out_filepath.exists() && !options.force {
        return Err(fmt!("File %s already exists.  Use -f to overwrite it.", gz_filepath));
    }

    match file::open(&out_filepath, Create, Write) {
        Some(writer_stream) => Ok(writer_stream),
        None => Err(~"Failed to open file for write.")
    }
}

fn compress_write_loop<R: Reader, W: Writer>(mut stream_reader: R, stream_writer: W, filepath: &Path, options: &Options) -> ~str {
    let file_name = filepath.filename().unwrap_or(&"");
    let mut mtime = 0u32;
    let mut file_size = 0u32;

    match file::stat(filepath) {
        Some(fs) => {
            mtime = (fs.modified / 1000) as u32;
            file_size = fs.size as u32;
        },
        None => ()
    }

    match GZipWriter::with_size_factor(stream_writer, file_name, mtime, file_size, options.compress_level, options.size_factor) {
        Ok(gzip_writer) => {
            let mut gzip_writer = gzip_writer;
            let mut input_buf = vec::from_elem(gzip::calc_buf_size(options.size_factor), 0u8);
            debug!(fmt!("compress_write_loop input_buf.len: %u", input_buf.len()));
            loop {
                match stream_reader.read(input_buf) {
                    Some(n) => {
                        debug!(fmt!("stream_reader.read: %u", n));
                        gzip_writer.write(input_buf.slice(0, n));
                    },
                    None    => {
                        debug!(fmt!("stream_reader.finalize"));
                        gzip_writer.finalize();
                        break;
                    }
                }
            }
            return ~"";
        },
        Err(s) =>
            return fmt!("%?", s)
    }
}

fn compress_pipe_loop<R: Reader, W: Writer>(mut stream_reader: R, mut stream_writer: W, filepath: &Path, options: &Options) -> ~str {
    let file_name = filepath.filename().unwrap_or(&"");
    let mut mtime = 0u32;
    let mut file_size = 0u32;

    match file::stat(filepath) {
        Some(fs) => {
            mtime = (fs.modified / 1000) as u32;
            file_size = fs.size as u32;
        },
        None => ()
    }

    match GZip::compress_init(&mut stream_writer, file_name, mtime, file_size) {
        Ok(gzip) => {
            let mut gzip = gzip;
            match gzip.compress_pipe(&mut stream_reader, &mut stream_writer, options.compress_level, options.size_factor) {
                Ok(_)   => ~"",
                Err(s)  => s
            }
        },
        Err(s) => s
    }
}

fn compress_file(options: &Options, file: &str) -> ~str {

    let filepath = Path(file);

    if filepath.filetype().unwrap_or("").to_ascii().to_lower().to_str_ascii().equals(&~".gz") {
        return fmt!("File %s already has the .gz suffix -- unchanged", file);
    }

    let mut result = ~"working";
    do io_error::cond.trap(|c| {
        println(fmt!("trap99 %?", c));
        fail2!(fmt!("trap %?", c));
    }).inside {
        let stream_reader = match file::open(&filepath, Open, Read) {
            Some(reader) => reader,
            None => fail2!("whoops! I'm sure this raised, anyways..")
        };
        let stream_writer = match open_compressed_writer(options, &filepath) {
            Ok(writer) => writer,
            Err(err) => fail2!(fmt!("%?", err))
        };

        result = if options.use_pipe {
            compress_pipe_loop(stream_reader, stream_writer, &filepath, options)
        } else {
            compress_write_loop(stream_reader, stream_writer, &filepath, options)
        };
    }

    result
}


fn open_decompressed_writer(options: &Options, filepath: &Path) -> Result<FileStream, ~str> {
    if options.stdout {
        //let writer = stdio::stdout();
        //return writer;
        fail2!("std::rt::io::stdout is not implemented yet");
    }

    let filestem = match filepath.filestem() {
        Some(stem) => stem,
        None => return Err(~"Not a file.")
    };

    let out_filepath = filepath.with_filename(filestem);
    match file::open(&out_filepath, Create, Write) {
        Some(writer_stream) => Ok(writer_stream),
        None => Err(~"Failed to open file for write.")
    }
}

fn decompress_read_loop<R: Reader, W: Writer>(stream_reader: R, mut stream_writer: W, buf_size_factor: uint) -> Result<uint, ~str> {
    match GZipReader::with_size_factor(stream_reader, buf_size_factor) {
        Ok(gzip_reader) => {
            let mut gzip_reader = gzip_reader;
            let mut out_buf = ~[0u8, ..DEFAULT_BUF_SIZE];
            loop {
                match gzip_reader.read(out_buf) {
                    Some(n) => stream_writer.write(out_buf.slice(0, n)),
                    None    => break
                }
            }
            stream_writer.flush();
            Ok(0)
        },
        Err(s) =>
            Err(fmt!("%?", s))
    }
}

fn decompress_pipe_loop<R: Reader, W: Writer>(mut stream_reader: R, mut stream_writer: W, buf_size_factor: uint) -> Result<uint, ~str> {
    match GZip::decompress_init(&mut stream_reader) {
        Ok(gzip) => {
            let mut gzip = gzip;
            match gzip.decompress_pipe(&mut stream_reader, &mut stream_writer, buf_size_factor) {
                Ok(_)   => Ok(0),
                Err(s)  => Err(s)
            }
        },
        Err(s) =>
            Err(fmt!("%?", s))
    }
}

fn decompress_file(options: &Options, file: &str) -> Result<uint, ~str> {

    let filepath = Path(file);

    // Check for valid filetype
    match filepath.filetype() {
        Some(filetype) => {
            if !filetype.to_ascii().to_lower().to_str_ascii().equals(&~".gz") {
                return Err(fmt!("Error: file %s does not have the .gz suffix.", file));
            }
        },
        None =>
            return Err(fmt!("Error: file %s has no .gz suffix.", file))
    }

    let mut result = Ok(0);
    do io_error::cond.trap(|c| {
        println(fmt!("trap %?", c));
        fail2!(fmt!("trap %?", c));
    }).inside {
        let stream_reader = match file::open(&filepath, Open, Read) {
            Some(reader) => reader,
            None => fail2!("whoops! I'm sure this raised, anyways..")
        };
        let stream_writer = match open_decompressed_writer(options, &filepath) {
            Ok(writer) => writer,
            Err(err) => fail2!(fmt!("%?", err))
        };

        result = if options.use_pipe {
            decompress_pipe_loop(stream_reader, stream_writer, options.size_factor)
        } else {
            decompress_read_loop(stream_reader, stream_writer, options.size_factor)
        };
    }

    result
}

fn list_file(options: &Options, file: &str) -> Result<~str, ~str> {

    let filepath = Path(file);

    // Check for valid filetype
    match filepath.filetype() {
        Some(filetype) => {
            if !filetype.to_ascii().to_lower().to_str_ascii().equals(&~".gz") {
                return Err(fmt!("Error: file %s does not have the .gz suffix.", file));
            }
        },
        None =>
            return Err(fmt!("Error: file %s has no .gz suffix.", file))
    };

    let file_size = match file::stat(&filepath) {
        Some(fs)    => fs.size,
        None        => 0
    };

    let mut result = Ok(~"");
    do io_error::cond.trap(|c| {
        println(fmt!("trap %?", c));
        fail2!(fmt!("trap %?", c));
    }).inside {
        let mut stream_reader = match file::open(&filepath, Open, Read) {
            Some(reader) => reader,
            None => fail2!("whoops! I'm sure this raised, anyways..")
        };

        result = match GZip::read_info(&mut stream_reader) {
            Ok(gzip) => {
                Ok(fmt!("%10u %10u %5.1f%% %s", file_size as uint, gzip.original_size as uint, (file_size as f64 * 100f64 / gzip.original_size as f64), gzip.filename.unwrap()))
            },
            Err(s) =>
                Err(fmt!("%?", s))
        };
    }

    return result;
}


fn print_str(text: &str) {
    if text.len() > 0 {
        println(text);
    }
}

fn print_err(result: Result<uint, ~str>) {
    match result {
        Ok(_)   => (),
        Err(s)  => println(s)
    }
}

fn print_result(result: Result<~str, ~str>) {
    match result {
        Ok(s)   => println(s),
        Err(s)  => println(s)
    }
}

fn main()  {

    let args = os::args();
    match Options::from_args(&args) {
        Ok(options) => {
            match options.cmd {
                HELP =>
                    print_usage(&args),
                VERSION =>
                    print_version(&args),
                COMPRESS => {
                    for file in options.files.iter() {
                        print_str(compress_file(&options, *file));
                    }
                },
                DECOMPRESS => {
                    for file in options.files.iter() {
                        print_err(decompress_file(&options, *file));
                    }
                },
                LIST => {
                    println("compressed uncompress  ratio uncompressed_name");
                    for file in options.files.iter() {
                        print_result(list_file(&options, *file));
                    }
                }
            }
        },
        Err(err) => {
            println(fmt!("\n%s\n", err));
            print_usage(&args);
        }
    }

}
