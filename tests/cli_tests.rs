// SPDX-FileCopyrightText: © 2025 TTKB, LLC
// SPDX-License-Identifier: BSD-3-CLAUSE

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tempfile::TempDir;

use psyk::cli;
use psyk::io;

const PSYQ_PREFIX: &str = "tests/data/psy-q";

#[test]
fn test_split_and_rejoin() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // Copy a test LIB file to temp directory
    let p = format!("{PSYQ_PREFIX}/3.3/PSX/LIB/LIBAPI.LIB");
    let test_lib = Path::new(&p);
    let temp_lib = temp_path.join("LIBAPI.LIB");
    fs::copy(test_lib, &temp_lib)?;

    // Split it
    let original_lib = io::read_lib(&temp_lib)?;
    let original_count = original_lib.modules().len();

    // Change to temp directory for split operation
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(temp_path)?;

    cli::split(&temp_lib)?;

    // Verify OBJ files were created
    for module in original_lib.modules() {
        let obj_file = temp_path.join(format!("{}.OBJ", module.name()));
        assert!(
            obj_file.exists(),
            "Expected {} to exist",
            obj_file.display()
        );
    }

    // Try to rejoin them
    let rejoined_lib = temp_path.join("REJOINED.LIB");
    let obj_files: Vec<PathBuf> = original_lib
        .modules()
        .iter()
        .map(|m| temp_path.join(format!("{}.OBJ", m.name())))
        .collect();

    cli::join(&rejoined_lib, obj_files)?;

    // Verify the rejoined library
    let rejoined = io::read_lib(&rejoined_lib)?;
    assert_eq!(rejoined.modules().len(), original_count);

    // Restore original directory
    std::env::set_current_dir(original_dir)?;

    // temp_dir automatically cleaned up here
    Ok(())
}

#[test]
fn test_info_lib() -> Result<()> {
    let p = format!("{PSYQ_PREFIX}/3.3/PSX/LIB/LIBSN.LIB");
    let mut output: Vec<u8> = Vec::new();

    cli::info(&mut output, Path::new(&p), false, false, false)?;

    assert_eq!("\
        Module     Date     Time   Externals defined\n\
        \n\
        OPEN     20-09-95 20:44:22 PCopen \n\
        CLOSE    20-09-95 20:44:22 PCclose \n\
        LSEEK    20-09-95 20:44:22 PClseek \n\
        CREAT    20-09-95 20:44:22 PCcreat \n\
        SNREAD   20-09-95 20:44:22 _SN_read \n\
        SNMAIN   20-09-95 20:44:22 __bss __heapsize __SN_ENTRY_POINT __bsslen __data __main __text __datalen __textlen __do_global_dtors __heapbase \n\
        FSINIT   20-09-95 20:44:22 PCinit \n\
        SNWRITE  20-09-95 20:44:22 _SN_write \n\
        READ     20-09-95 20:44:24 PCread \n\
        WRITE    20-09-95 20:44:24 PCwrite \n\
        SNDEF    20-09-95 20:44:24 _stacksize _ramsize \n\
        PUREV    20-09-95 20:44:24 __pure_virtual \n\
        CACHE    20-09-95 20:44:24 SNFlushCache \n\
        _ASHLDI3 20-09-95 20:44:26 __ashldi3 \n\
        _ASHRDI3 20-09-95 20:44:26 __ashrdi3 \n\
        _CMPDI2  20-09-95 20:44:28 __cmpdi2 \n\
        _DIVDI3  20-09-95 20:44:28 __divdi3 \n\
        _EH      20-09-95 20:44:30 __throw_type_match __register_exceptions __find_first_exception_table_match \n\
        _FFSDI2  20-09-95 20:44:30 __ffsdi2 \n\
        _FXDFDI  20-09-95 20:44:32 __fixdfdi \n\
        _FXSFDI  20-09-95 20:44:32 __fixsfdi \n\
        _FXTFDI  20-09-95 20:44:34 \n\
        _FXUSDFD 20-09-95 20:44:34 __fixunsdfdi \n\
        _FIXUSDF 20-09-95 20:44:36 __fixunsdfsi \n\
        _FXUSSFD 20-09-95 20:44:36 __fixunssfdi \n\
        _FXUSSFS 20-09-95 20:44:38 __fixunssfsi \n\
        _FXUSTFD 20-09-95 20:44:38 \n\
        _FXUSXFD 20-09-95 20:44:38 \n\
        _FXUSXFS 20-09-95 20:44:40 \n\
        _FXXFDI  20-09-95 20:44:40 \n\
        _FLTDIDF 20-09-95 20:44:42 __floatdidf \n\
        _FLTDISF 20-09-95 20:44:42 __floatdisf \n\
        _FLTDITF 20-09-95 20:44:44 \n\
        _FLTDIXF 20-09-95 20:44:44 \n\
        _LSHLDI3 20-09-95 20:44:46 __lshldi3 \n\
        _LSHRDI3 20-09-95 20:44:46 __lshrdi3 \n\
        _MODDI3  20-09-95 20:44:48 __moddi3 \n\
        _MULDI3  20-09-95 20:44:48 __muldi3 \n\
        _NEGDI2  20-09-95 20:44:50 __negdi2 \n\
        _NEW_HAN 20-09-95 20:44:50 set_new_handler __new_handler __default_new_handler \n\
        _OP_DELE 20-09-95 20:44:50 __builtin_delete \n\
        _OP_NEW  20-09-95 20:44:52 __builtin_new \n\
        _OP_VDEL 20-09-95 20:44:52 __builtin_vec_delete \n\
        _OP_VNEW 20-09-95 20:44:54 __builtin_vec_new \n\
        _SHTAB   20-09-95 20:44:54 __shtab \n\
        _TRAMPOL 20-09-95 20:44:56 \n\
        _UCMPDI2 20-09-95 20:44:56 __ucmpdi2 \n\
        _UDIVDI3 20-09-95 20:44:56 __udivdi3 \n\
        _UDIVMOD 20-09-95 20:45:00 __udivmoddi4 \n\
        _UDIV_W_ 20-09-95 20:45:00 __udiv_w_sdiv \n\
        _UMODDI3 20-09-95 20:45:02 __umoddi3 \n\
        _VARARGS 20-09-95 20:45:02 __builtin_saveregs \n\
        __GCC_BC 20-09-95 20:45:04 __gcc_bcmp \n\
        \n\
    ", String::from_utf8(output).expect("output").as_str());

    Ok(())
}

#[test]
fn test_info_obj() -> Result<()> {
    let p = format!("{PSYQ_PREFIX}/3.3/PSX/LIB/2MBYTE.OBJ");
    let mut output: Vec<u8> = Vec::new();

    cli::info(&mut output, Path::new(&p), false, false, false)?;

    assert_eq!(
        "\
        Header : LNK version 2\n\
        46 : Processor type 7\n\
        16 : Section symbol number 2808 '.rdata' in group 0 alignment 8\n\
        16 : Section symbol number 2809 '.text' in group 0 alignment 8\n\
        16 : Section symbol number 280a '.data' in group 0 alignment 8\n\
        16 : Section symbol number 280b '.sdata' in group 0 alignment 8\n\
        16 : Section symbol number 280c '.sbss' in group 0 alignment 8\n\
        16 : Section symbol number 280d '.bss' in group 0 alignment 8\n\
        6 : Switch to section 2808\n\
        6 : Switch to section 2809\n\
        6 : Switch to section 280a\n\
        6 : Switch to section 280b\n\
        6 : Switch to section 280c\n\
        6 : Switch to section 280d\n\
        6 : Switch to section 2809\n\
        2 : Code 196 bytes\n\
        10 : Patch type 82 at offset 8 with sectstart(280c)\n\
        10 : Patch type 84 at offset c with sectstart(280c)\n\
        10 : Patch type 82 at offset 10 with sectend(280d)\n\
        10 : Patch type 84 at offset 14 with sectend(280d)\n\
        10 : Patch type 82 at offset 40 with (sectbase(2809)+$b4)\n\
        10 : Patch type 84 at offset 44 with (sectbase(2809)+$b4)\n\
        10 : Patch type 82 at offset 58 with sectend(280d)\n\
        10 : Patch type 84 at offset 5c with sectend(280d)\n\
        10 : Patch type 82 at offset 68 with [2817]\n\
        10 : Patch type 84 at offset 6c with [2817]\n\
        10 : Patch type 82 at offset 80 with (sectbase(280c)+$0)\n\
        10 : Patch type 84 at offset 84 with (sectbase(280c)+$0)\n\
        10 : Patch type 82 at offset 88 with sectstart(280b)\n\
        10 : Patch type 84 at offset 8c with sectstart(280b)\n\
        10 : Patch type 74 at offset 94 with [2814]\n\
        10 : Patch type 82 at offset 9c with (sectbase(280c)+$0)\n\
        10 : Patch type 84 at offset a0 with (sectbase(280c)+$0)\n\
        10 : Patch type 74 at offset a8 with [2816]\n\
        6 : Switch to section 280c\n\
        8 : Uninitialized data, 4 bytes\n\
        14 : XREF symbol number 2814 'InitHeap'\n\
        14 : XREF symbol number 2817 '_stacksize'\n\
        12 : XDEF symbol number 280f '__SN_ENTRY_POINT' at offset 8 in section 2809\n\
        12 : XDEF symbol number 280e '__main' at offset 0 in section 2809\n\
        14 : XREF symbol number 2816 'main'\n\
        12 : XDEF symbol number 2811 'stup0' at offset a8 in section 2809\n\
        12 : XDEF symbol number 2812 'stup1' at offset 2c in section 2809\n\
        12 : XDEF symbol number 2813 'stup2' at offset 8 in section 2809\n\
        0 : End of file\n\
        \n\
    ",
        String::from_utf8(output).expect("output").as_str()
    );

    Ok(())
}
