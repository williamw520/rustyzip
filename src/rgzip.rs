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


extern mod extra;
extern mod rustyzip;

use std::os;
use std::num;
use std::vec;
use std::result::{Result, Ok, Err};
use extra::getopts::{optflag, optopt, getopts};
use std::path::Path;
use std::rt::io::file;
use std::rt::io::file::FileStream;
use std::rt::io::{Reader, Writer, Open, Create, Read, Write, io_error};

use rustyzip::rustyzip_lib::gzip;
use rustyzip::rustyzip_lib::gzip::{GZip, GZipReader, GZipWriter};



static VERSION_STR : &'static str = "0.8";


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
            name: false,
            quiet: false,
            verbose: false,
            compress_level: gzip::DEFAULT_COMPRESS_LEVEL,
            use_pipe: true,
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
                options.name = matches.opt_present("N") || matches.opt_present("name");
                options.quiet = matches.opt_present("q") || matches.opt_present("quiet");
                options.verbose = matches.opt_present("v") || matches.opt_present("verbose");
                for level in range(0u, 10u) {
                    let slevel = fmt!("%u", level);
                    options.compress_level = if matches.opt_present(slevel) { level } else { options.compress_level };
                }
                options.use_pipe = !matches.opt_present("Pipe");
                let mut size_factor = if matches.opt_present("bufsize") { maybe_to_num(matches.opt_str("bufsize"), gzip::DEFAULT_SIZE_FACTOR) } else { gzip::DEFAULT_SIZE_FACTOR };
                size_factor = if matches.opt_present("b")               { maybe_to_num(matches.opt_str("b"), size_factor) } else { size_factor };
                options.size_factor = num::max(gzip::MIN_SIZE_FACTOR, size_factor);
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
    path.filestem().unwrap_or(&"").to_owned()
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
        None => Err(fmt!("Failed to open file %s for write.", out_filepath.to_str()))
    }
}

fn compress_pipe_loop<R: Reader, W: Writer>(mut stream_reader: R, mut stream_writer: W, filepath: &Path, options: &Options) -> ~str {
    let file_name = if options.no_name { &"" } else { filepath.filename().unwrap_or(&"") };
    let mut mtime = 0u32;
    let mut file_size = 0u32;

    match file::stat(filepath) {
        Some(fs) => {
            mtime = if options.no_name { 0u32 } else { (fs.modified / 1000) as u32 };
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
            ~""
        },
        Err(s) =>
            s
    }
}

fn compress_file(options: &Options, file: &str) -> ~[~str] {
    let mut results : ~[~str] = ~[];

    let filepath = Path(file);
    if filepath.filetype().unwrap_or("").to_ascii().to_lower().to_str_ascii().equals(&~".gz") {
        results.push(fmt!("File %s already has the .gz suffix -- unchanged", file));
        return results;
    }

    do io_error::cond.trap(|c| {
        results.push(c.to_str());
    }).inside {
        match file::open(&filepath, Open, Read) {
            Some(stream_reader) => {
                match open_compressed_writer(options, &filepath) {
                    Ok(stream_writer) => {
                        let result = if options.use_pipe {
                            compress_pipe_loop(stream_reader, stream_writer, &filepath, options)
                        } else {
                            compress_write_loop(stream_reader, stream_writer, &filepath, options)
                        };
                        results.push(result);
                    },
                    Err(errstr) => 
                        results.push(fmt!("%s %s", errstr, filepath.to_str()))
                }
            },
            None => 
                results.push(fmt!("Failed to open file %s", filepath.to_str()))
        }
    }

    results
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

fn decompress_pipe_loop<R: Reader>(mut stream_reader: R, out_file: &str, options: &Options) -> ~str {
    match GZip::decompress_init(&mut stream_reader) {
        Ok(gzip) => {
            let decomp_filename = if options.name { gzip.filename.clone().unwrap_or(out_file.to_owned()) } else { out_file.to_owned() };
            let decomp_filepath = Path(decomp_filename);
            match open_decompressed_writer(options, &decomp_filepath) {
                Ok(stream_writer) => {
                    let mut stream_writer = stream_writer;
                    let mut gzip = gzip;
                    match gzip.decompress_pipe(&mut stream_reader, &mut stream_writer, options.size_factor) {
                        Ok(_)   => ~"",
                        Err(s)  => s
                    }
                },
                Err(errstr) => 
                    fmt!("%s %s", errstr, decomp_filepath.to_str())
            }
        },
        Err(s) => s
    }
}

fn decompress_read_loop<R: Reader>(stream_reader: R, out_file: &str, options: &Options) -> ~str {
    match GZipReader::with_size_factor(stream_reader, options.size_factor) {
        Ok(gzip_reader) => {
            let decomp_filename = if options.name { gzip_reader.gzip.filename.clone().unwrap_or(out_file.to_owned()) } else { out_file.to_owned() };
            let decomp_filepath = Path(decomp_filename);
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
                    fmt!("%s %s", errstr, decomp_filepath.to_str())
            }
            
        },
        Err(s) => s
    }
}

fn decompress_file(options: &Options, file: &str) -> ~[~str] {
    let mut results : ~[~str] = ~[];

    // Check for valid filetype
    let filepath = Path(file);
    match filepath.filetype() {
        Some(filetype) => {
            if !filetype.to_ascii().to_lower().to_str_ascii().equals(&~".gz") {
                results.push(fmt!("File %s does not have the .gz suffix.  No action.", file))
            }
        },
        None =>
            results.push(fmt!("File %s has no .gz suffix.  No action.", file))
    };
    if results.len() > 0 {
        return results;
    }

    do io_error::cond.trap(|c| {
        results.push(c.to_str());
    }).inside {
        match file::open(&filepath, Open, Read) {
            Some(stream_reader) => {
                let result = if options.use_pipe {
                    decompress_pipe_loop(stream_reader, file, options)
                } else {
                    decompress_read_loop(stream_reader, file, options)
                };
                results.push(result);
            },
            None => 
                results.push(fmt!("Failed to open file %s", filepath.to_str()))
        }
    }
    results
}

fn list_file(file: &str) -> ~[~str] {
    let mut results : ~[~str] = ~[];

    // Check for valid filetype
    let filepath = Path(file);
    match filepath.filetype() {
        Some(filetype) => {
            if !filetype.to_ascii().to_lower().to_str_ascii().equals(&~".gz") {
                results.push(fmt!("File %s does not have the .gz suffix.  No action.", file))
            }
        },
        None =>
            results.push(fmt!("File %s has no .gz suffix.  No action.", file))
    };
    if results.len() > 0 {
        return results;
    }

    let file_size = match file::stat(&filepath) {
        Some(fs)    => fs.size,
        None        => 0
    };

    do io_error::cond.trap(|c| {
        results.push(c.to_str());
    }).inside {
        match file::open(&filepath, Open, Read) {
            Some(stream_reader) => {
                let mut stream_reader = stream_reader;
                match GZip::read_info(&mut stream_reader) {
                    Ok(gzip) => {
                        results.push(fmt!("%10u  %10u %5.1f%%  %s", 
                                          file_size as uint, 
                                          gzip.original_size as uint, 
                                          (file_size as f64 * 100f64 / gzip.original_size as f64), 
                                          gzip.filename.unwrap_or(~"")));
                    },
                    Err(errstr) =>
                        results.push(fmt!("%s %s", errstr, filepath.to_str()))
                }
            },
            None => 
                results.push(fmt!("Failed to open file %s", filepath.to_str()))
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
            println(fmt!("\n%s\n", err));
            print_usage(&args);
        }
    }
}
