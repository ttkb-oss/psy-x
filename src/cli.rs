// SPDX-FileCopyrightText: © 2025 TTKB, LLC
// SPDX-License-Identifier: BSD-3-CLAUSE

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{File, FileTimes};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::crate_version;

use super::display;
use super::io::{read, read_lib, write_lib, write_obj};
use super::{Module, LIB};

/// Prints information about an [OBJ](super::OBJ) or [LIB].
pub fn info(
    write: &mut impl Write,
    lib_or_obj: &Path,
    code: bool,
    disassembly: bool,
    recursive: bool,
) -> Result<()> {
    let o = read(lib_or_obj)?;
    let mut options = display::Options::default();
    if disassembly {
        options.code_format = display::CodeFormat::Disassembly;
    } else if code {
        options.code_format = display::CodeFormat::Hex;
    }
    options.recursive = recursive;
    writeln!(write, "{}", display::PsyXDisplayable::wrap(&o, options))?;
    Ok(())
}

pub fn split(lib_path: &Path) -> Result<()> {
    let lib = read_lib(lib_path)?;
    println!("psyk version {}\n", crate_version!());
    for module in lib.modules() {
        let object_filename = format!("{}.OBJ", module.name());
        let time = module.created_at().expect("created timestamp");
        let mut file = File::create(&object_filename)?;
        let times = FileTimes::new().set_accessed(time).set_modified(time);
        file.set_times(times)?;
        write_obj(module.object(), &mut file)?;

        println!("Extracted object file {}", object_filename);
    }
    Ok(())
}

pub fn delete(lib_path: &Path, obj_names: Vec<String>) -> Result<()> {
    let lib = read_lib(lib_path)?;

    let module_names: HashSet<String> = HashSet::from_iter(obj_names);

    let new_modules: Vec<Module> = lib
        .modules()
        .iter()
        .filter(|m| !module_names.contains(&m.name()))
        .cloned()
        .collect::<Vec<Module>>();
    let lib = LIB::new(new_modules);

    let mut file = File::create(lib_path)?;
    write_lib(&lib, &mut file)
}

pub fn join(lib_path: &Path, obj_paths: Vec<PathBuf>) -> Result<()> {
    let modules = obj_paths
        .iter()
        .map(|path| Module::new_from_path(path).expect("module"))
        .collect::<Vec<Module>>();

    let lib = LIB::new(modules);

    let mut file = File::create(lib_path)?;
    write_lib(&lib, &mut file)
}

pub fn add(lib_path: &Path, obj_path: &Path) -> Result<()> {
    let lib = read_lib(lib_path)?;

    let module = Module::new_from_path(obj_path)?;
    let mut modules: Vec<Module> = lib.modules().clone();
    modules.push(module);

    let lib = LIB::new(modules);

    let mut file = File::create(lib_path)?;
    write_lib(&lib, &mut file)
}

pub fn update(lib_path: &Path, obj_paths: Vec<PathBuf>) -> Result<()> {
    let lib = read_lib(lib_path)?;

    let mut updated_module_paths: HashMap<String, PathBuf> = HashMap::new();
    for path in obj_paths {
        let module_name = String::from(path.file_stem().expect("file").to_string_lossy());
        updated_module_paths.insert(module_name, path);
    }

    let new_modules = lib
        .modules()
        .iter()
        .map({
            |m| {
                if let Some(module_path) = updated_module_paths.get(&m.name()) {
                    let Ok(new_mod) = Module::new_from_path(module_path) else {
                        eprintln!("could not read: {module_path:?}. Skipping.");
                        return m.clone();
                    };
                    new_mod
                } else {
                    m.clone()
                }
            }
        })
        .collect::<Vec<Module>>();
    let lib = LIB::new(new_modules);

    let mut file = File::create(lib_path)?;
    write_lib(&lib, &mut file)
}

fn stem_or_psyk(path: Option<String>) -> String {
    path.and_then(|path| {
        Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
    })
    .unwrap_or_else(|| "psyk".to_string())
}

/// Get the binary name from the executable path
pub fn get_binary_name() -> String {
    stem_or_psyk(env::args().next())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bin_name() {
        assert_eq!("psyk", stem_or_psyk(None));
        assert_eq!("foo", stem_or_psyk(Some("/bin/foo".into())));
    }
}
