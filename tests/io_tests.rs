// SPDX-FileCopyrightText: © 2025 TTKB, LLC
// SPDX-License-Identifier: BSD-3-CLAUSE

use std::fs::File;
use std::path::Path;
use std::time::SystemTime;

use psyk::io;
use psyk::Module;
use psyk::ModuleMetadata;
use psyk::Section;
use psyk::LIB;
use psyk::OBJ;

const PSYQ_PREFIX: &str = "tests/data/psy-q";

#[test]
fn test_lib_creation() {
    let lib =
        io::read_lib(Path::new(&format!("{PSYQ_PREFIX}/3.5/PSX/LIB/LIBCD.LIB"))).expect("lib");

    let modules = lib.modules();

    let new_lib = LIB::new(modules.clone());

    assert_eq!(lib, new_lib);
}

#[test]
fn test_bad_filenames() {
    assert!(io::read(Path::new("bad file name")).is_err());
    assert!(io::read_obj(Path::new("bad file name")).is_err());
    assert!(io::read_lib(Path::new("bad file name")).is_err());
}

#[test]
fn test_not_readable() {
    assert!(io::read(Path::new("/dev/fd")).is_err());
    assert!(io::read_obj(Path::new("/dev/fd")).is_err());
    assert!(io::read_lib(Path::new("/dev/fd")).is_err());
}

#[test]
fn test_file_too_small() {
    let e = io::read(Path::new("/dev/null")).expect_err("error");
    assert_eq!(
        Some("File too small to contain valid PSY-Q magic number"),
        e.chain().next().map(|x| format!("{x}")).as_deref()
    );
}

#[test]
fn test_bad_files() {
    let e = io::read(Path::new("tests/data/truncated.txt")).expect_err("error");
    assert_eq!(
        Some("Unrecognized magic [116, 120, 116]"),
        e.chain().next().map(|x| format!("{x}")).as_deref()
    );

    let e = io::read(Path::new("tests/data/truncated.lib")).expect_err("error");
    assert_eq!(
        Some("assertion failed: `! objs.is_empty()` at 0x0"),
        e.chain().next().map(|x| format!("{x}")).as_deref()
    );

    let e = io::read(Path::new("tests/data/truncated.obj")).expect_err("error");
    let msg = e.chain().next().map(|x| format!("{x}")).expect("reason");
    assert!(msg.starts_with(" 0: Error: no variants matched at 0x4..."));
}

#[test]
fn test_bad_obj_files() {
    let e = io::read_obj(Path::new("tests/data/truncated.txt")).expect_err("error");
    assert_eq!(
        Some("bad magic at 0x0: [116, 120, 116]"),
        e.chain().next().map(|x| format!("{x}")).as_deref()
    );

    let e = io::read_obj(Path::new("tests/data/truncated.lib")).expect_err("error");
    assert_eq!(
        Some("bad magic at 0x0: [76, 73, 66]"),
        e.chain().next().map(|x| format!("{x}")).as_deref()
    );

    let e = io::read_obj(Path::new("tests/data/truncated.obj")).expect_err("error");
    let msg = e.chain().next().map(|x| format!("{x}")).expect("reason");
    assert!(msg.starts_with(" 0: Error: no variants matched at 0x4..."));
}

#[test]
fn test_bad_lib_files() {
    let e = io::read_lib(Path::new("tests/data/truncated.txt")).expect_err("error");
    assert_eq!(
        Some("bad magic at 0x0: [116, 120, 116]"),
        e.chain().next().map(|x| format!("{x}")).as_deref()
    );

    let e = io::read_lib(Path::new("tests/data/truncated.lib")).expect_err("error");
    assert_eq!(
        Some("assertion failed: `! objs.is_empty()` at 0x0"),
        e.chain().next().map(|x| format!("{x}")).as_deref()
    );

    let e = io::read_lib(Path::new("tests/data/truncated.obj")).expect_err("error");
    assert_eq!(
        Some("bad magic at 0x0: [76, 78, 75]"),
        e.chain().next().map(|x| format!("{x}")).as_deref()
    );
}

#[test]
fn test_write_errors() {
    let obj = OBJ::new(vec![Section::NOP]);
    let metadata = ModuleMetadata::new("foo".to_string(), SystemTime::now(), 5, vec![]);
    let module = Module::new(obj.clone(), metadata);
    let lib = LIB::new(vec![module]);

    let mut file = File::open("/dev/fd").expect("file");

    let e = io::write_obj(&obj.clone(), &mut file).expect_err("error");
    assert_eq!(
        "Bad file descriptor (os error 9)",
        e.chain().next().map(|x| format!("{x}")).as_deref().unwrap()
    );

    let e = io::write_lib(&lib.clone(), &mut file).expect_err("error");
    assert_eq!(
        "Bad file descriptor (os error 9)",
        e.chain().next().map(|x| format!("{x}")).as_deref().unwrap()
    );
}
