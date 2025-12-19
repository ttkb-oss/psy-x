// SPDX-FileCopyrightText: © 2025 TTKB, LLC
// SPDX-License-Identifier: BSD-3-CLAUSE

use std::env;
use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};

mod dos;

use psyk::cli::{self, get_binary_name};

/// Inspect, extract, and create PSY-Q LIB and OBJ files.
#[derive(Debug, Parser)]
#[clap(name = env!("CARGO_CRATE_NAME"), version)]
#[command(version, about, long_about = None)]
pub struct App {
    #[arg(required = false)]
    lib_or_obj: Option<PathBuf>,

    #[clap(subcommand)]
    command: Option<CLICommand>,
}

#[derive(Debug, Subcommand)]
enum CLICommand {
    /// List the contents of the LIB or OBJ
    List {
        /// a LIB or OBJ file
        #[arg(required = true)]
        lib_or_obj: PathBuf,

        /// enable a listing of code in the dump
        #[clap(short, long)]
        code: bool,

        /// show disassembly of code for known architectures
        #[clap(short, long)]
        disassemble: bool,

        /// recursively print all OBJ entries in a LIB
        #[clap(short, long)]
        recursive: bool,
    },

    /// splits a LIB into multiple OBJs
    Extract {
        /// the LIB to extract
        #[arg(required = true)]
        lib: PathBuf,
    },

    /// Create a new LIB containing provided OBJs into a LIB
    Create {
        /// the LIB to create
        #[arg(required = true)]
        lib: PathBuf,
        /// the OBJs to include
        #[arg(num_args=1..)]
        objs: Vec<PathBuf>,
    },

    /// Adds an OBJ into an existing LIB
    Add {
        /// the LIB to modify
        #[arg(required = true)]
        lib: PathBuf,
        /// the OBJ to add
        #[arg(required = true)]
        obj: PathBuf,
    },

    /// Updates one or more OBJs in an existing LIB
    Update {
        /// the LIB to modify
        #[arg(required = true)]
        lib: PathBuf,
        /// the OBJs to update
        #[arg(num_args=1..)]
        objs: Vec<PathBuf>,
    },

    /// Updates one or more OBJs in an existing LIB
    Delete {
        /// the LIB to modify
        #[arg(required = true)]
        lib: PathBuf,
        /// the OBJs to delete
        #[arg(num_args=1..)]
        obj_names: Vec<String>,
    },
}

fn main() -> Result<()> {
    match get_binary_name().as_str() {
        "dumpobj" => return dos::dumpobj_main(),
        "psylib" => return dos::psylib_main(),
        _ => (),
    }

    let args = App::parse();

    match args.command {
        Some(command) => match command {
            CLICommand::List {
                lib_or_obj,
                code,
                disassemble,
                recursive,
            } => cli::info(
                &mut std::io::stdout(),
                &lib_or_obj,
                code,
                disassemble,
                recursive,
            )?,
            CLICommand::Extract { lib } => cli::split(&lib)?,
            CLICommand::Create { lib, objs } => cli::join(&lib, objs)?,
            CLICommand::Add { lib, obj } => cli::add(&lib, &obj)?,
            CLICommand::Update { lib, objs } => cli::update(&lib, objs)?,
            CLICommand::Delete { lib, obj_names } => cli::delete(&lib, obj_names)?,
        },
        None => match args.lib_or_obj {
            Some(lib_or_obj) => {
                cli::info(&mut std::io::stdout(), &lib_or_obj, false, false, false)?
            }
            None => {
                let a = App::command().render_help();
                eprintln!("{}", a);
            }
        },
    }

    Ok(())
}
