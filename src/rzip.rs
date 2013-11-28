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
extern mod rustyzip;
use rustyzip::gzip;
use rustyzip::zip;
use rustyzip::zip::{ZipFile};

// Uncomment these to use the modules in the system's libextra.
// use extra::gzip;
// use extra::gzip::{GZip, GZipReader, GZipWriter};



use std::os;
use std::num;
use std::result::{Result, Ok, Err};
use std::to_str::ToStr;
use std::path::Path;
use std::io::{Open, Read, io_error};
use std::io::fs::File;
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


fn list_file(file: &str) -> ~[~str] {
    let mut results : ~[~str] = ~[];

    // Check for valid filetype
    let filepath = Path::new(file);
    match filepath.extension_str() {
        Some(filetype) => {
            if !filetype.to_ascii().to_lower().into_str().equals(&~"zip") {
                results.push(format!("File {:s} does not have the .zip suffix.  No action.", file))
            }
        },
        None =>
            results.push(format!("File {:s} has no .zip suffix.  No action.", file))
    };
    if results.len() > 0 {
        return results;
    }

    io_error::cond.trap(|c| {
        results.push(c.to_str());
    }).inside(|| {
        match File::open_mode(&filepath, Open, Read) {
            Some(stream_reader) => {
                match ZipFile::open(stream_reader) {
                    Ok(zipfile) => {
                        let mut zipfile = zipfile;
                        
                        let entries = zipfile.get_zip_entries().unwrap();
                        for ze in entries.iter() {
                            println(format!("{:?}\r\n", ze));
                        }
                    }
                    Err(errstr) =>
                        results.push(format!("{:s} {:s}", errstr, filepath.as_str().unwrap_or("")))
                }
            },
            None => 
                results.push(format!("Failed to open file {:s}", filepath.as_str().unwrap_or("")))
        }
    });

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
                LIST => {
                    for file in options.files.iter() {
                        print_lines(list_file(*file));
                    }
                },
                _ => ()

            }
        },
        Err(err) => {
            println(format!("\n{:s}\n", err));
            print_usage(&args);
        }
    }
}
