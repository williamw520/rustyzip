// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0.  If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
// 
// Software distributed under the License is distributed on an "AS IS" basis,
// WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for 
// the specific language governing rights and limitations under the License.
//
// The Original Code is: rgzip.rs
// The Initial Developer of the Original Code is: William Wong (williamw520@gmail.com)
// Portions created by William Wong are Copyright (C) 2013 William Wong, All Rights Reserved.


extern mod extra;


// The gzip and deflate code exist in two places: in the original RustyZip project library and in the Rust's 'extra' runtime library.
// Uncomment either one of the following sections to link to one or the other library.

// Uncomment these to use the local modules in the local rustyzip.lib.
// extern mod rustyzip;
// use rustyzip::gzip;
// use rustyzip::gzip::{GZip, GZipReader, GZipWriter};

// Uncomment these to use the modules in the system's libextra.
use extra::gzip;
use extra::gzip::{GZip, GZipReader, GZipWriter};



use std::os;
use std::num;
use std::vec;
use std::result::{Result, Ok, Err};
use std::to_str::ToStr;
use std::path::Path;
use std::rt::io;
use std::rt::io::{Reader, Writer, Open, Read, Truncate, Write, io_error};
use std::rt::io::fs;
use std::rt::io::fs::File;
use extra::getopts::{optflag, optopt, getopts};



static VERSION_STR : &'static str = "0.9";


enum Cmd {
    HELP, VERSION, COMPRESS, DECOMPRESS, LIST
}

struct Options {
    cmd:            Cmd,
    stdout:         bool,
    force:          bool,
    no_name:        bool,
    name:           bool,
    quiet:          bool,
    verbose:        bool,
    compress_level: uint,
    use_stream:     bool,
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
            name: false,
            quiet: false,
            verbose: false,
            compress_level: gzip::DEFAULT_COMPRESS_LEVEL,
            use_stream: true,
            size_factor: gzip::DEFAULT_SIZE_FACTOR,
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
                     optflag("N"),
                     optflag("name"),
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
                     optflag("Stream"),
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
                options.name = matches.opt_present("N") || matches.opt_present("name");
                options.quiet = matches.opt_present("q") || matches.opt_present("quiet");
                options.verbose = matches.opt_present("v") || matches.opt_present("verbose");
                for level in range(0u, 10u) {
                    let slevel = format!("{:u}", level);
                    options.compress_level = if matches.opt_present(slevel) { level } else { options.compress_level };
                }
                options.use_stream = !matches.opt_present("Stream");
                let mut size_factor = if matches.opt_present("bufsize") { maybe_to_num(matches.opt_str("bufsize"), gzip::DEFAULT_SIZE_FACTOR) } else { gzip::DEFAULT_SIZE_FACTOR };
                size_factor = if matches.opt_present("b")               { maybe_to_num(matches.opt_str("b"), size_factor) } else { size_factor };
                options.size_factor = num::max(gzip::MIN_SIZE_FACTOR, size_factor);
                options.files = matches.free;

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
    let path: Path = Path::new((*args)[0].to_owned());
    match path.filestem_str() {
        Some(s) => s.to_str(),
        None    => ~""
    }
}

fn print_usage(args: &~[~str]) {
    println(format!("Usage: {:s}  -h --help -d --decompress -c --stdout FILE ...", get_program(args)));
}

fn print_version(args: &~[~str]) {
    println(format!("{0:s} {1:s}", get_program(args), VERSION_STR));
    println("Written by William Wong");
}

fn get_file_name(filepath: &Path) -> ~str {
    match filepath.filename_str() {
            Some(f) => f.to_str(), 
            None => ~""
    }
}


fn open_compressed_writer(options: &Options, file: &str) -> Result<File, ~str> {
    if options.stdout {
        //let writer = stdio::stdout();
        //return writer;
        fail!("std::rt::io::stdout is not implemented yet");
    }

    let gz_filepath = file + ".gz";
    let out_filepath = Path::new(gz_filepath.clone());
    if out_filepath.exists() && !options.force {
        return Err(format!("File {:s} already exists.  Use -f to overwrite it.", gz_filepath));
    }

    match File::open_mode(&out_filepath, Truncate, Write) {
        Some(writer_stream) => Ok(writer_stream),
        None => Err(format!("Failed to open file {:?} for write.", out_filepath))
    }
}

fn compress_stream_loop<R: Reader, W: Writer>(mut stream_reader: R, mut stream_writer: W, filepath: &Path, options: &Options) -> ~str {
    let file_name = if options.no_name { ~"" } else { get_file_name(filepath) };
    let mut mtime;
    let mut file_size;

    match io::result(|| fs::stat(filepath)) {
        Ok(stat) => {
            mtime = if options.no_name { 0u32 } else { (stat.modified / 1000) as u32 };
            file_size = stat.size as u32;
        },
        Err(e) => {
            return format!("{:?}", e);
        }
    }

    match GZip::compress_init(&mut stream_writer, file_name, mtime, file_size) {
        Ok(gzip) => {
            let mut gzip = gzip;
            match gzip.compress_stream(&mut stream_reader, &mut stream_writer, options.compress_level, options.size_factor) {
                Ok(_)   => ~"",
                Err(s)  => s
            }
        },
        Err(s) => s
    }
}

fn compress_write_loop<R: Reader, W: Writer>(mut stream_reader: R, stream_writer: W, filepath: &Path, options: &Options) -> ~str {
    let file_name = get_file_name(filepath);
    let mut mtime;
    let mut file_size;

    match io::result(|| fs::stat(filepath)) {
        Ok(stat) => {
            mtime = if options.no_name { 0u32 } else { (stat.modified / 1000) as u32 };
            file_size = stat.size as u32;
        },
        Err(e) => {
            return format!("{:?}", e);
        }
    }

    match GZipWriter::with_size_factor(stream_writer, file_name, mtime, file_size, options.compress_level, options.size_factor) {
        Ok(gzip_writer) => {
            let mut gzip_writer = gzip_writer;
            let mut input_buf = vec::from_elem(gzip::calc_buf_size(options.size_factor), 0u8);
            loop {
                match stream_reader.read(input_buf) {
                    Some(n) => {
                        gzip_writer.write(input_buf.slice(0, n));
                    },
                    None    => {
                        gzip_writer.finalize();
                        break;
                    }
                }
            }
            ~""
        },
        Err(s) =>
            s
    }
}

fn compress_file(options: &Options, file: &str) -> ~[~str] {
    let mut results : ~[~str] = ~[];

    let filepath = Path::new(file);
    if filepath.extension_str().unwrap_or("").to_ascii().to_lower().to_str_ascii().equals(&~"gz") {
        results.push(format!("File {:s} already has the .gz suffix -- unchanged", file));
        return results;
    }

    do io_error::cond.trap(|c| {
        results.push(c.to_str());
    }).inside {
        match File::open_mode(&filepath, Open, Read) {
            Some(stream_reader) => {
                match open_compressed_writer(options, file) {
                    Ok(stream_writer) => {
                        let result = if options.use_stream {
                            compress_stream_loop(stream_reader, stream_writer, &filepath, options)
                        } else {
                            compress_write_loop(stream_reader, stream_writer, &filepath, options)
                        };
                        results.push(result);
                    },
                    Err(errstr) => 
                        results.push(format!("{0:s} {1:s}", errstr, filepath.as_str().unwrap_or("")))
                }
            },
            None => 
                results.push(format!("Failed to open file {:s}", filepath.as_str().unwrap_or("")))
        }
    }

    results
}


fn open_decompressed_writer(options: &Options, filepath: &Path) -> Result<File, ~str> {
    if options.stdout {
        //let writer = stdio::stdout();
        //return writer;
        fail!("std::rt::io::stdout is not implemented yet");
    }

    let filestem = match filepath.filestem_str() {
        Some(stem) => stem,
        None => return Err(~"Not a file.")
    };

    let out_filepath = filepath.with_filename(filestem);
    if out_filepath.exists() && !options.force {
        return Err(~"File already exists.  Use -f to overwrite it.");
    }
    match File::open_mode(&out_filepath, Truncate, Write) {
        Some(writer_stream) => Ok(writer_stream),
        None => Err(~"Failed to open file for write.")
    }
}

fn decompress_stream_loop<R: Reader>(mut stream_reader: R, out_file: &str, options: &Options) -> ~str {
    match GZip::decompress_init(&mut stream_reader) {
        Ok(gzip) => {
            let decomp_filename = if options.name { gzip.filename.clone().unwrap_or(out_file.to_owned()) } else { out_file.to_owned() };
            let decomp_filepath = Path::new(decomp_filename);
            match open_decompressed_writer(options, &decomp_filepath) {
                Ok(stream_writer) => {
                    let mut stream_writer = stream_writer;
                    let mut gzip = gzip;
                    match gzip.decompress_stream(&mut stream_reader, &mut stream_writer, options.size_factor) {
                        Ok(_)   => ~"",
                        Err(s)  => s
                    }
                },
                Err(errstr) => 
                    format!("{0:s} {1:s}", errstr, decomp_filepath.as_str().unwrap_or(""))
            }
        },
        Err(s) => s
    }
}

fn decompress_read_loop<R: Reader>(stream_reader: R, out_file: &str, options: &Options) -> ~str {
    match GZipReader::with_size_factor(stream_reader, options.size_factor) {
        Ok(gzip_reader) => {
            let decomp_filename = if options.name { gzip_reader.gzip.filename.clone().unwrap_or(out_file.to_owned()) } else { out_file.to_owned() };
            let decomp_filepath = Path::new(decomp_filename);
            match open_decompressed_writer(options, &decomp_filepath) {
                Ok(stream_writer) => {
                    let mut stream_writer = stream_writer;
                    let mut gzip_reader = gzip_reader;
                    let mut out_buf = vec::from_elem(gzip::calc_buf_size(options.size_factor), 0u8);
                    loop {
                        match gzip_reader.read(out_buf) {
                            Some(n) => stream_writer.write(out_buf.slice(0, n)),
                            None    => break
                        }
                    }
                    stream_writer.flush();
                    ~""
                },
                Err(errstr) => 
                    format!("{:s} {:s}", errstr, decomp_filepath.as_str().unwrap_or(""))
            }
            
        },
        Err(s) => s
    }
}

fn decompress_file(options: &Options, file: &str) -> ~[~str] {
    let mut results : ~[~str] = ~[];

    // Check for valid filetype
    let filepath = Path::new(file);
    match filepath.extension_str() {
        Some(filetype) => {
            if !filetype.to_ascii().to_lower().to_str_ascii().equals(&~"gz") {
                results.push(format!("File {:s} does not have the .gz suffix.  No action.", file))
            }
        },
        None =>
            results.push(format!("File {:s} has no .gz suffix.  No action.", file))
    };
    if results.len() > 0 {
        return results;
    }

    do io_error::cond.trap(|c| {
        results.push(c.to_str());
    }).inside {
        match File::open_mode(&filepath, Open, Read) {
            Some(stream_reader) => {
                let result = if options.use_stream {
                    decompress_stream_loop(stream_reader, file, options)
                } else {
                    decompress_read_loop(stream_reader, file, options)
                };
                results.push(result);
            },
            None => 
                results.push(format!("Failed to open file {:s}", filepath.as_str().unwrap_or("")))
        }
    }
    results
}

fn list_file(file: &str) -> ~[~str] {
    let mut results : ~[~str] = ~[];

    // Check for valid filetype
    let filepath = Path::new(file);
    match filepath.extension_str() {
        Some(filetype) => {
            if !filetype.to_ascii().to_lower().to_str_ascii().equals(&~"gz") {
                results.push(format!("File {:s} does not have the .gz suffix.  No action.", file))
            }
        },
        None =>
            results.push(format!("File {:s} has no .gz suffix.  No action.", file))
    };
    let mut file_size: u64;
    match io::result(|| fs::stat(&filepath)) {
        Ok(stat) => {
            file_size = stat.size;
        },
        Err(_) => {
            file_size = 0;
        }
    }
    if results.len() > 0 {
        return results;
    }

    do io_error::cond.trap(|c| {
        results.push(c.to_str());
    }).inside {
        match File::open_mode(&filepath, Open, Read) {
            Some(stream_reader) => {
                let mut stream_reader = stream_reader;
                match GZip::read_info(&mut stream_reader) {
                    Ok(gzip) => {
                        results.push(format!("{:10u}  {:10u} {:5.1f}%  {:s}", 
                                          file_size as uint, 
                                          gzip.original_size as uint, 
                                          (file_size as f64 * 100f64 / gzip.original_size as f64), 
                                          gzip.filename.unwrap_or(~"")));
                    },
                    Err(errstr) =>
                        results.push(format!("{:s} {:s}", errstr, filepath.as_str().unwrap_or("")))
                }
            },
            None => 
                results.push(format!("Failed to open file {:s}", filepath.as_str().unwrap_or("")))
        }
    }

    results
}


fn print_lines(lines: ~[~str]) {
    for line in lines.iter() {
        if line.len() > 0 {
            println(*line);
        }
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
                        print_lines(compress_file(&options, *file));
                    }
                },
                DECOMPRESS => {
                    for file in options.files.iter() {
                        print_lines(decompress_file(&options, *file));
                    }
                },
                LIST => {
                    println("compressed  uncompress  ratio  uncompressed_name");
                    for file in options.files.iter() {
                        print_lines(list_file(*file));
                    }
                }
            }
        },
        Err(err) => {
            println(format!("\n{:s}\n", err));
            print_usage(&args);
        }
    }
}
