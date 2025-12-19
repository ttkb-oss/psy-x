// SPDX-FileCopyrightText: © 2025 TTKB, LLC
// SPDX-License-Identifier: BSD-3-CLAUSE

//! A module for providing a DOS-like interface to `psyk`.

use std::env;
use std::path::PathBuf;
use std::process;

use anyhow::{bail, Result};

use crate::cli;
use psyk::{display, io};

fn dumpobj_usage() -> ! {
    let args: Vec<String> = env::args().collect();
    eprintln!("Usage: {} <file> [/c] [/d]", args[0]);
    eprintln!();
    eprintln!("Options:");
    eprintln!("  /c    Show code listing");
    eprintln!("  /d    Show disassembly");
    process::exit(1);
}

/// Alternate main that accepts DOS-style arguments.
///
/// Usage:
/// - `program file.obj` - basic info
/// - `program file.obj /c` - info with code listing
/// - `program file.obj /d` - info with disassembly
pub fn dumpobj_main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut options = display::Options::default();

    let obj_path = match args.len() {
        2 => PathBuf::from(args[1].clone()),
        3 => {
            match args[1].as_str() {
                "/c" => options.code_format = display::CodeFormat::Hex,
                "/d" => options.code_format = display::CodeFormat::Disassembly,
                _ => dumpobj_usage(),
            }
            PathBuf::from(args[2].clone())
        }
        _ => dumpobj_usage(),
    };

    // info
    let obj = io::read(&obj_path)?;
    println!("{}", display::PsyXDisplayable::wrap(&obj, options));
    Ok(())
}

fn psylib_usage() -> ! {
    let args: Vec<String> = env::args().collect();
    eprintln!("Usage: {} <option> <library> ...", args[0]);
    eprintln!("Usage: {} /a add modules", args[0]);
    eprintln!("       {} /d delete modules []", args[0]);
    eprintln!("       {} /u <library.lib> <obj1> [obj2...]", args[0]);
    eprintln!("       {} /x <library.lib>", args[0]);
    eprintln!("       {} /l <library.lib >", args[0]);
    process::exit(1);
}

/// - `psylib /x file.lib` - split library
/// - `psylib /a output.lib file1.obj file2.obj` - join objects
pub fn psylib_main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        psylib_usage();
    }

    // Check for subcommands
    match args[1].to_lowercase().as_str() {
        "/a" => {
            if args.len() < 4 {
                bail!("Usage: {} /a <library> <obj>", args[0]);
            }
            return cli::add(&PathBuf::from(&args[2]), &PathBuf::from(&args[3]));
        }
        "/d" => {
            if args.len() < 4 {
                bail!("Usage: {} /d <output.lib> <obj1> [obj2...]", args[0]);
            }
            let lib_path = PathBuf::from(&args[2]);
            let obj_name = args[3].clone();

            return cli::delete(&lib_path, [obj_name].to_vec());
        }
        "/u" => {
            if args.len() < 4 {
                bail!("Usage: {} /u <output.lib> <obj1> [obj2...]", args[0]);
            }
            let lib_path = &PathBuf::from(&args[2]);
            let obj_paths: Vec<PathBuf> = args[3..].iter().map(PathBuf::from).collect();
            return cli::update(lib_path, obj_paths);
        }
        "/x" => {
            if args.len() < 3 {
                bail!("Usage: {} /x <library>", args[0]);
            }
            return cli::split(&PathBuf::from(&args[2]));
        }
        "/l" => {
            if args.len() < 3 {
                bail!("Usage: {} /l <library>", args[0]);
            }
            return cli::info(
                &mut std::io::stdout(),
                &PathBuf::from(&args[2]),
                false,
                false,
                false,
            );
        }
        _ => {}
    }

    Ok(())
}
