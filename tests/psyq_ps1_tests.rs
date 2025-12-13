// SPDX-FileCopyrightText: © 2025 TTKB, LLC
// SPDX-License-Identifier: BSD-3-CLAUSE

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use psyk::io;
use psyk::Module;
use psyk::Section;
use serde_json::{self};

use std::sync::LazyLock;

mod common;

use common::{compare_output, round_trip};

static LIB_EXPORTS: LazyLock<HashMap<String, HashMap<String, HashSet<String>>>> =
    LazyLock::new(|| {
        serde_json::from_str(
            r#"{
        "LIBAPI": {
            "A56": ["exit"],
            "C57": ["InitHeap"],
            "C58": ["_exit"],
            "C65": ["LoadTest"],
            "C66": ["Load"],
            "C67": ["Exec"],
            "C68": ["FlushCache"],
            "C73": ["GPU_cw"],
            "C81": ["LoadExec"],
            "C82": ["GetSysSp"],
            "C112": ["_bu_init"],
            "C113": ["_96_init"],
            "C114": ["_96_remove"],
            "C156": ["SetConf"],
            "C157": ["GetConf"],
            "C159": ["SetMem"],
            "C160": ["_boot"],
            "C161": ["SystemError"],
            "C167": ["bufs_cb_0"],
            "C168": ["bufs_cb_1"],
            "C169": ["bufs_cb_2"],
            "C170": ["bufs_cb_3"],
            "C174": ["bufs_cb_4"],
            "A07": ["DeliverEvent"],
            "A08": ["OpenEvent"],
            "A09": ["CloseEvent"],
            "A10": ["WaitEvent"],
            "A11": ["TestEvent"],
            "A12": ["EnableEvent"],
            "A13": ["DisableEvent"],
            "A14": ["OpenTh"],
            "A15": ["CloseTh"],
            "A16": ["ChangeTh"],
            "A19": ["StartPAD"],
            "A20": ["StopPAD"],
            "A21": ["PAD_init"],
            "A22": ["PAD_dr"],
            "A23": ["ReturnFromException"],
            "A24": ["ResetEntryInt"],
            "A25": ["HookEntryInt"],
            "A32": ["UnDeliverEvent"],
            "A36": ["EnterCriticalSection"],
            "A37": ["ExitCriticalSection"],
            "A38": ["Exception"],
            "A39": ["SetSp"],
            "A40": ["SwEnterCriticalSection"],
            "A41": ["SwExitCriticalSection"],
            "A50": ["open"],
            "A51": ["lseek"],
            "A52": ["read"],
            "A53": ["write"],
            "A54": ["close"],
            "A55": ["ioctl"],
            "A64": ["cd"],
            "A65": ["format"],
            "A66": ["firstfile"],
            "A67": ["nextfile"],
            "A68": ["rename"],
            "A69": ["delete"],
            "A70": ["undelete"],
            "A71": ["AddDrv"],
            "A72": ["DelDrv"],
            "A81": ["Krom2RawAdd"],
            "A84": ["_get_errno"],
            "A85": ["_get_error"],
            "A91": ["ChangeClearPAD"],
            "A94": ["GetGp"],
            "A95": ["GetSp"],
            "A96": ["GetCr"],
            "A97": ["GetSr"],
            "L02": ["SysEnqIntRP"],
            "L03": ["SysDeqIntRP"],
            "L10": ["ChangeClearRCnt"],
            "COUNTER": ["SetRCnt", "GetRCnt", "StartRCnt", "StopRCnt", "ResetRCnt"],
            "SC2B": ["SetConf"],
            "__MAIN":   [],
            "PATCH":    ["_patch_pad"],
            "PAD":      ["InitPAD", "StartPAD", "PAD_init"],
            "I_HEAP2":  ["_AllocRestBlockTop InitHeap2 _max_heap _Candidate _TopAllocArea"],
            "MALLOC2":  ["malloc2"],
            "FREE2":    ["free2"],
            "REALLOC2": ["realloc2"],
            "CALLOC2":  ["calloc2"]
        },
        "LIBC": {
            "A56": ["exit"],
            "A58": ["getc"],
            "A59": ["putc"],
            "A60": ["getchar"],
            "A61": ["putchar"],
            "A62": ["gets"],
            "A63": ["puts"],
            "C10": ["todigit"],
            "C12": ["strtoul"],
            "C13": ["strtol"],
            "C14": ["abs"],
            "C15": ["labs"],
            "C16": ["atoi"],
            "C17": ["atol"],
            "C18": ["atob"],
            "C19": ["setjmp"],
            "C20": ["longjmp"],
            "C21": ["strcat"],
            "C22": ["strncat"],
            "C23": ["strcmp"],
            "C24": ["strncmp"],
            "C25": ["strcpy"],
            "C26": ["strncpy"],
            "C27": ["strlen"],
            "C28": ["index"],
            "C29": ["rindex"],
            "C30": ["strchr"],
            "C31": ["strrchr"],
            "C32": ["strpbrk"],
            "C33": ["strspn"],
            "C34": ["strcspn"],
            "C36": ["strstr"],
            "C37": ["toupper"],
            "C38": ["tolower"],
            "C39": ["bcopy"],
            "C40": ["bzero"],
            "C42": ["memcpy"],
            "C43": ["memset"],
            "C46": ["memchr"],
            "C47": ["rand"],
            "C48": ["srand"],
            "C49": ["qsort"],
            "C51": ["malloc"],
            "C52": ["free"],
            "C53": ["lsearch"],
            "C54": ["bsearch"],
            "C55": ["calloc"],
            "C56": ["realloc"],
            "C63": ["printf"],
            "CTYPE0": ["_ctype_"],
            "SPRINTF": ["sprintf"],
            "ITOA": ["itoa"],
            "MEMMOVE": ["memmove"],
            "BCMP": ["bcmp"],
            "MEMCMP": ["memcmp"],
            "STRTOK": ["strtok"]
        },
        "LIBC2": {
            "ABS": ["abs"],
            "ATOI": ["atoi"],
            "BCMP": ["bcmp"],
            "BCOPY": ["bcopy"],
            "BZERO": ["bzero"],
            "CTYPE": ["tolower"],
            "GETC": ["getc"],
            "GETCHAR": ["getchar"],
            "GETS": ["gets"],
            "MEMCHR": ["memchr"],
            "MEMCMP": ["memcmp"],
            "MEMCPY": ["memcpy"],
            "MEMMOVE": ["memmove"],
            "MEMSET": ["memset"],
            "PUTC": ["putc"],
            "PUTCHAR": ["putchar"],
            "QSORT": ["qsort"],
            "RAND": ["srand"],
            "STRCAT": ["strcat"],
            "STRCHR": ["strchr"],
            "STRCMP": ["strcmp"],
            "STRCPY": ["strcpy"],
            "STRCSPN": ["strcspn"],
            "STRINGS": ["index"],
            "STRLEN": ["strlen"],
            "STRNCAT": ["strncat"],
            "STRNCMP": ["strncmp"],
            "STRNCPY": ["strncpy"],
            "STRPBRK": ["strpbrk"],
            "STRRCHR": ["strrchr"],
            "STRSPN": ["strspn"],
            "STRSTR": ["strstr"],
            "STRTOK": ["strtok"],
            "STRTOL": ["atob"],
            "STRTOUL": ["strtoul"],
            "TODIGIT": ["todigit"],
            "PRINTF": ["printf"],
            "PRNT": ["prnt"],
            "SPRINTF": ["sprintf"],
            "ITOA": ["itoa"],
            "PUTS": ["puts"],
            "BSEARCH": ["bsearch"],
            "LSEARCH": ["lsearch"],
            "SETJMP": ["setjmp"],
            "EXIT": ["exit"],
            "MALLOC": ["malloc"]
        }
    }"#,
        )
        .expect("export maps")
    });

pub fn test_exports(module: &Module, lib_name: &str) {
    // TODO: later SDKs moved pad functions into the PAD module
    if module.name() == "A18"
        || module.name() == "A19"
        || module.name() == "A20"
        || module.name() == "A21"
    {
        // A18 should export InitPad, but some versions of the
        // SDK export InitPad2
        return;
    }

    if module.name() == "A69" || module.name() == "I_HEAP2" {
        return;
    }

    let Some(library_exports) = LIB_EXPORTS.get(lib_name) else {
        eprintln!("export tests not configured for {lib_name}");
        return;
    };

    let Some(exports) = library_exports.get(&module.name()) else {
        eprintln!(
            "export tests not configured for {lib_name}:{}",
            module.name()
        );
        return;
    };

    let mut found_exports = HashSet::new();

    for section in &module.object().sections {
        let symbol = match section {
            Section::XREF(xref) => Some(xref.symbol_name()),
            Section::XDEF(xdef) => Some(xdef.symbol_name()),
            _ => None,
        };
        if let Some(symbol) = symbol {
            found_exports.insert(symbol.clone());
        }
    }

    assert!(
        exports.is_subset(&found_exports),
        "{lib_name}:{} expected all exports ({exports:?}) in {found_exports:?}\nOBJ: {}",
        module.name(),
        module.object(),
    );
}

pub fn test(path: &Path) {
    let Ok(io::Type::LIB(lib)) = io::read(path) else {
        return;
    };

    for entry in lib.modules() {
        test_exports(entry, path.file_stem().unwrap().to_str().unwrap());
    }
}

const PRIVATE_TEST_DATA_PREFIX: &str = "target/.private/tests/data";
const PSYQ_PREFIX: &str = "tests/data/psy-q";

#[inline]
fn path_version(version: &str, file: &str) -> PathBuf {
    PathBuf::from(format!("{PSYQ_PREFIX}/{version}/{file}"))
}

#[inline]
fn path_33(file: &str) -> PathBuf {
    path_version("3.3", file)
}

#[inline]
fn path_35(file: &str) -> PathBuf {
    path_version("3.5", file)
}

#[inline]
fn path_36(file: &str) -> PathBuf {
    path_version("3.6", file)
}

#[inline]
fn path_40(file: &str) -> PathBuf {
    path_version("4.0", file)
}

#[inline]
fn path_46(file: &str) -> PathBuf {
    PathBuf::from(format!(
        "{PRIVATE_TEST_DATA_PREFIX}/Psy-Q_46/Psy-Q - 46/{file}"
    ))
}

#[inline]
fn path_47(file: &str) -> PathBuf {
    PathBuf::from(format!("{PRIVATE_TEST_DATA_PREFIX}/Psy-Q_47/{file}"))
}

#[test]
fn test_psyq_33() {
    round_trip(&path_33("PSX/LIB/2MBYTE.OBJ"));
    round_trip(&path_33("PSX/LIB/8MBYTE.OBJ"));
    round_trip(&path_33("PSX/LIB/MALLOC.OBJ"));
    round_trip(&path_33("PSX/LIB/NONE2.OBJ"));
    round_trip(&path_33("PSX/LIB/NONE3.OBJ"));
    round_trip(&path_33("PSX/SAMPLE/MODULE/EXECMENU/FONTTEX1.OBJ"));
    // n.b.! there is an extra XDEF at the end of this OBJ
    // a NULL byte at 0x2554 acts as the EOF marker even
    // though there is one additional section in the file
    // DUMPOBJ.EXE does not find this section, either.
    //round_trip(&path_33("PSX/UTILITY/MENU/CDSFILE.OBJ"));

    round_trip(&path_33("PSX/LIB/LIBAPI.LIB"));
    round_trip(&path_33("PSX/LIB/LIBC.LIB"));
    round_trip(&path_33("PSX/LIB/LIBC2.LIB"));
    round_trip(&path_33("PSX/LIB/LIBCARD.LIB"));
    round_trip(&path_33("PSX/LIB/LIBCD.LIB"));
    round_trip(&path_33("PSX/LIB/LIBCOMB.LIB"));
    round_trip(&path_33("PSX/LIB/LIBETC.LIB"));
    round_trip(&path_33("PSX/LIB/LIBGPU.LIB"));
    round_trip(&path_33("PSX/LIB/LIBGS.LIB"));
    round_trip(&path_33("PSX/LIB/LIBGTE.LIB"));
    round_trip(&path_33("PSX/LIB/LIBMATH.LIB"));
    round_trip(&path_33("PSX/LIB/LIBPRESS.LIB"));
    round_trip(&path_33("PSX/LIB/LIBSN.LIB"));
    round_trip(&path_33("PSX/LIB/LIBSND.LIB"));
    round_trip(&path_33("PSX/LIB/LIBSPU.LIB"));
    round_trip(&path_33("PSX/LIB/LIBTAP.LIB"));
    round_trip(&path_33("PSX/SAMPLE/ETC/CARD/LIB/SUPERX.LIB"));
    round_trip(&path_33("PSX/SAMPLE/ETC/CARD/LIB/TURTLE.LIB"));
}

#[test]
fn test_psyq_35() {
    round_trip(&path_35("PSX/LIB/2MBYTE.OBJ"));
    round_trip(&path_35("PSX/LIB/8MBYTE.OBJ"));
    round_trip(&path_35("PSX/LIB/MALLOC.OBJ"));
    round_trip(&path_35("PSX/LIB/NONE2.OBJ"));
    round_trip(&path_35("PSX/LIB/NONE3.OBJ"));
    //round_trip(&path_35("PSYQ/SRC/SYMMUNGE/SYMMUNGE.OBJ")); // coff file

    round_trip(&path_35("PSX/LIB/LIBAPI.LIB"));
    round_trip(&path_35("PSX/LIB/LIBC.LIB"));
    round_trip(&path_35("PSX/LIB/LIBC2.LIB"));
    round_trip(&path_35("PSX/LIB/LIBCARD.LIB"));
    round_trip(&path_35("PSX/LIB/LIBCD.LIB"));
    round_trip(&path_35("PSX/LIB/LIBCOMB.LIB"));
    round_trip(&path_35("PSX/LIB/LIBETC.LIB"));
    round_trip(&path_35("PSX/LIB/LIBGPU.LIB"));
    round_trip(&path_35("PSX/LIB/LIBGS.LIB"));
    round_trip(&path_35("PSX/LIB/LIBGTE.LIB"));
    round_trip(&path_35("PSX/LIB/LIBGUN.LIB"));
    round_trip(&path_35("PSX/LIB/LIBMATH.LIB"));
    round_trip(&path_35("PSX/LIB/LIBPRESS.LIB"));
    round_trip(&path_35("PSX/LIB/LIBSN.LIB"));
    round_trip(&path_35("PSX/LIB/LIBSND.LIB"));
    round_trip(&path_35("PSX/LIB/LIBSPU.LIB"));
    round_trip(&path_35("PSX/LIB/LIBTAP.LIB"));
}

#[test]
fn test_psyq_36() {
    round_trip(&path_36("PSX/LIB/2MBYTE.OBJ"));
    round_trip(&path_36("PSX/LIB/8MBYTE.OBJ"));
    round_trip(&path_36("PSX/LIB/NONE2.OBJ"));
    round_trip(&path_36("PSX/LIB/NONE3.OBJ"));
    round_trip(&path_36("PSX/SAMPLE/SCEE/DEMODISC/DEMO/NONE2/NONE2.OBJ"));
    round_trip(&path_36("PSX/SAMPLE/SCEE/SUBDIV/MAIN.OBJ"));
    round_trip(&path_36("PSX/SAMPLE/SCEE/SUBDIV/SUBDIV.OBJ"));
    round_trip(&path_36("PSX/UTILITY/MENU/MENU.OBJ"));
    round_trip(&path_36("PSX/UTILITY/MENU/PCEXEC.OBJ"));
    round_trip(&path_36("PSX/UTILITY/MENU/PCLOAD.OBJ"));
    round_trip(&path_36("PSX/UTILITY/MENU/PRINTERR.OBJ"));
    round_trip(&path_36("PSX/UTILITY/MENU/SDATA.OBJ"));
    round_trip(&path_36("PSX/UTILITY/MENU/SOUND.OBJ"));
    round_trip(&path_36("PSX/UTILITY/MENU/STRING.OBJ"));
    //round_trip(&path_36("PSYQ/SRC/SYMMUNGE/SYMMUNGE.OBJ")); // coff file

    round_trip(&path_36("PSX/LIB/LIBAPI.LIB"));
    round_trip(&path_36("PSX/LIB/LIBC.LIB"));
    round_trip(&path_36("PSX/LIB/LIBC2.LIB"));
    round_trip(&path_36("PSX/LIB/LIBCARD.LIB"));
    round_trip(&path_36("PSX/LIB/LIBCD.LIB"));
    round_trip(&path_36("PSX/LIB/LIBCOMB.LIB"));
    round_trip(&path_36("PSX/LIB/LIBETC.LIB"));
    round_trip(&path_36("PSX/LIB/LIBGPU.LIB"));
    round_trip(&path_36("PSX/LIB/LIBGS.LIB"));
    round_trip(&path_36("PSX/LIB/LIBGTE.LIB"));
    round_trip(&path_36("PSX/LIB/LIBGUN.LIB"));
    round_trip(&path_36("PSX/LIB/LIBMATH.LIB"));
    round_trip(&path_36("PSX/LIB/LIBPRESS.LIB"));
    round_trip(&path_36("PSX/LIB/LIBSIO.LIB"));
    round_trip(&path_36("PSX/LIB/LIBSN.LIB"));
    round_trip(&path_36("PSX/LIB/LIBSND.LIB"));
    round_trip(&path_36("PSX/LIB/LIBSPU.LIB"));
    round_trip(&path_36("PSX/LIB/LIBTAP.LIB"));
    round_trip(&path_36("PSX/SAMPLE/ETC/CARD/LIB/SUPERX.LIB"));
    round_trip(&path_36("PSX/SAMPLE/ETC/CARD/LIB/TURTLE.LIB"));
    round_trip(&path_36("PSX/SAMPLE/SCEE/DEMODISC/DEMO/BS/BS.LIB"));
    round_trip(&path_36("PSX/SAMPLE/SCEE/DEMODISC/DEMO/NONE2/NONE2.LIB"));
    round_trip(&path_36("PSX/SAMPLE/SCEE/ETC/MTAP/LIBTAP.LIB"));
    round_trip(&path_36("PSX/UTILITY/MENU/CDSFILE.LIB"));
    round_trip(&path_36("PSYQ/BIN/LIBDECI.LIB"));
    round_trip(&path_36("PSYQ/SRC/LIBSN/LIBSN.LIB"));
}

#[test]
fn test_psyq_40() {
    round_trip(&path_40("PSX/LIB/2MBYTE.OBJ"));
    round_trip(&path_40("PSX/LIB/8MBYTE.OBJ"));
    round_trip(&path_40("PSX/LIB/AUTOPAD.OBJ"));
    round_trip(&path_40("PSX/LIB/NOHEAP.OBJ"));
    round_trip(&path_40("PSX/LIB/NONE3.OBJ"));
    round_trip(&path_40("PSX/LIB/POWERON.OBJ"));

    round_trip(&path_40("PSX/LIB/LIBAPI.LIB"));
    round_trip(&path_40("PSX/LIB/LIBC.LIB"));
    round_trip(&path_40("PSX/LIB/LIBC2.LIB"));
    round_trip(&path_40("PSX/LIB/LIBCARD.LIB"));
    round_trip(&path_40("PSX/LIB/LIBCD.LIB"));
    round_trip(&path_40("PSX/LIB/LIBCOMB.LIB"));
    round_trip(&path_40("PSX/LIB/LIBDS.LIB"));
    round_trip(&path_40("PSX/LIB/LIBETC.LIB"));
    round_trip(&path_40("PSX/LIB/LIBGPU.LIB"));
    round_trip(&path_40("PSX/LIB/LIBGS.LIB"));
    round_trip(&path_40("PSX/LIB/LIBGTE.LIB"));
    round_trip(&path_40("PSX/LIB/LIBGUN.LIB"));
    round_trip(&path_40("PSX/LIB/LIBMATH.LIB"));
    round_trip(&path_40("PSX/LIB/LIBMCRD.LIB"));
    round_trip(&path_40("PSX/LIB/LIBPRESS.LIB"));
    round_trip(&path_40("PSX/LIB/LIBSIO.LIB"));
    round_trip(&path_40("PSX/LIB/LIBSN.LIB"));
    round_trip(&path_40("PSX/LIB/LIBSND.LIB"));
    round_trip(&path_40("PSX/LIB/LIBSPU.LIB"));
    round_trip(&path_40("PSX/LIB/LIBTAP.LIB"));
    round_trip(&path_40("PSX/SAMPLE/GRAPHICS/ZIMEN/LIBZIMEN.LIB"));
    round_trip(&path_40("PSX/SAMPLE/OLD/ETC/CARD/LIB/SUPERX.LIB"));
    round_trip(&path_40("PSX/SAMPLE/OLD/ETC/CARD/LIB/TURTLE.LIB"));
    round_trip(&path_40("PSX/UTILITY/MENU/CDSFILE.LIB"));
    round_trip(&path_40("PSYQ/PREFSMPL/LIBGS2/LIBGS.LIB"));
}

#[test]
fn test_psyq_46() {
    round_trip(&path_46("LIB/LIBSN.LIB"));
    round_trip(&path_46("LIB/2MBYTE.OBJ"));
    round_trip(&path_46("LIB/LIBGTE.LIB"));
    round_trip(&path_46("LIB/LIBSPU.LIB"));
    round_trip(&path_46("LIB/8MBYTE.OBJ"));
    round_trip(&path_46("LIB/LIBDS.LIB"));
    round_trip(&path_46("LIB/LIBC.LIB"));
    round_trip(&path_46("LIB/LIBMATH.LIB"));
    round_trip(&path_46("LIB/LIBETC.LIB"));
    round_trip(&path_46("LIB/LIBCOMB.LIB"));
    round_trip(&path_46("LIB/LIBSND.LIB"));
    round_trip(&path_46("LIB/NONE3.OBJ"));
    round_trip(&path_46("LIB/LIBPRESS.LIB"));
    round_trip(&path_46("LIB/LIBAPI.LIB"));
    round_trip(&path_46("LIB/LIBC2.LIB"));
    round_trip(&path_46("LIB/LIBCD.LIB"));
    round_trip(&path_46("LIB/MCGUI_E.OBJ"));
    round_trip(&path_46("LIB/LIBMCRD.LIB"));
    round_trip(&path_46("LIB/LIBGPU.LIB"));
    round_trip(&path_46("LIB/LIBHMD.LIB"));
    round_trip(&path_46("LIB/LIBCARD.LIB"));
    round_trip(&path_46("LIB/NOPRINT.OBJ"));
    round_trip(&path_46("LIB/LIBTAP.LIB"));
    round_trip(&path_46("LIB/MCGUI.OBJ"));
    round_trip(&path_46("LIB/LIBMCX.LIB"));
    round_trip(&path_46("LIB/LIBGS.LIB"));
    round_trip(&path_46("LIB/LIBPAD.LIB"));
    round_trip(&path_46("LIB/LIBGUN.LIB"));
    round_trip(&path_46("LIB/LIBSIO.LIB"));
    round_trip(&path_46("LIB/NOHEAP.OBJ"));
}

#[test]
fn test_psyq_47() {
    round_trip(&path_47("LIB/POWERON.OBJ"));
    round_trip(&path_47("LIB/2MBYTE.OBJ"));
    round_trip(&path_47("LIB/LIBGTE.LIB"));
    round_trip(&path_47("LIB/LIBSPU.LIB"));
    round_trip(&path_47("LIB/8MBYTE.OBJ"));
    round_trip(&path_47("LIB/LIBDS.LIB"));
    round_trip(&path_47("LIB/LIBC.LIB"));
    round_trip(&path_47("LIB/LIBMATH.LIB"));
    round_trip(&path_47("LIB/LIBETC.LIB"));
    round_trip(&path_47("LIB/LIBCOMB.LIB"));
    round_trip(&path_47("LIB/LIBSND.LIB"));
    round_trip(&path_47("LIB/NONE3.OBJ"));
    round_trip(&path_47("LIB/LIBPRESS.LIB"));
    round_trip(&path_47("LIB/LIBAPI.LIB"));
    round_trip(&path_47("LIB/LIBC2.LIB"));
    round_trip(&path_47("LIB/MCGUI_E.OBJ"));
    round_trip(&path_47("LIB/LIBMCRD.LIB"));
    round_trip(&path_47("LIB/LIBGPU.LIB"));
    round_trip(&path_47("LIB/LIBHMD.LIB"));
    round_trip(&path_47("LIB/LIBCARD.LIB"));
    round_trip(&path_47("LIB/NOPRINT.OBJ"));
    round_trip(&path_47("LIB/LIBTAP.LIB"));
    round_trip(&path_47("LIB/MCGUI.OBJ"));
    round_trip(&path_47("LIB/LIBMCX.LIB"));
    round_trip(&path_47("LIB/LIBGS.LIB"));
    round_trip(&path_47("LIB/LIBPAD.LIB"));
    round_trip(&path_47("LIB/LIBGUN.LIB"));
    round_trip(&path_47("LIB/LIBSIO.LIB"));
    round_trip(&path_47("LIB/NOHEAP.OBJ"));
}

const CMD_DATA_PREFIX: &str = "tests/data/cmd/psy-q-psx";

#[inline]
fn cmd_output_version(version: &str, file: String) -> PathBuf {
    PathBuf::from(format!("{CMD_DATA_PREFIX}/{version}/{file}"))
}

#[inline]
fn cmd_output_33(file: String) -> PathBuf {
    cmd_output_version("3.3", file)
}

#[inline]
fn cmd_output_35(file: String) -> PathBuf {
    cmd_output_version("3.5", file)
}

#[inline]
fn cmd_output_36(file: String) -> PathBuf {
    cmd_output_version("3.6", file)
}

#[inline]
fn cmd_output_40(file: String) -> PathBuf {
    cmd_output_version("4.0", file)
}

fn compare_lib_output_33(prefix: &str) {
    compare_output(
        &path_33(&format!("{prefix}.LIB")),
        &cmd_output_33(format!("{prefix}.TXT")),
        3,
    );
}

fn compare_obj_output_33(prefix: &str) {
    compare_output(
        &path_33(&format!("{prefix}.OBJ")),
        &cmd_output_33(format!("{prefix}.TXT")),
        1,
    );
}

fn compare_lib_output_35(prefix: &str) {
    compare_output(
        &path_35(&format!("{prefix}.LIB")),
        &cmd_output_35(format!("{prefix}.TXT")),
        3,
    );
}

fn compare_obj_output_35(prefix: &str) {
    compare_output(
        &path_35(&format!("{prefix}.OBJ")),
        &cmd_output_35(format!("{prefix}.TXT")),
        1,
    );
}

fn compare_lib_output_36(prefix: &str) {
    compare_output(
        &path_36(&format!("{prefix}.LIB")),
        &cmd_output_36(format!("{prefix}.TXT")),
        3,
    );
}

fn compare_obj_output_36(prefix: &str) {
    compare_output(
        &path_36(&format!("{prefix}.OBJ")),
        &cmd_output_36(format!("{prefix}.TXT")),
        1,
    );
}

fn compare_lib_output_40(prefix: &str) {
    compare_output(
        &path_40(&format!("{prefix}.LIB")),
        &cmd_output_40(format!("{prefix}.TXT")),
        3,
    );
}

fn compare_obj_output_40(prefix: &str) {
    compare_output(
        &path_40(&format!("{prefix}.OBJ")),
        &cmd_output_40(format!("{prefix}.TXT")),
        1,
    );
}

#[test]
fn test_output_33() {
    compare_lib_output_33("PSX/LIB/LIBAPI");
    compare_lib_output_33("PSX/LIB/LIBC");
    compare_lib_output_33("PSX/LIB/LIBC2");
    compare_lib_output_33("PSX/LIB/LIBCARD");
    compare_lib_output_33("PSX/LIB/LIBCD");
    compare_lib_output_33("PSX/LIB/LIBCOMB");
    compare_lib_output_33("PSX/LIB/LIBETC");
    compare_lib_output_33("PSX/LIB/LIBGPU");
    compare_lib_output_33("PSX/LIB/LIBGS");
    compare_lib_output_33("PSX/LIB/LIBGTE");
    compare_lib_output_33("PSX/LIB/LIBMATH");
    compare_lib_output_33("PSX/LIB/LIBPRESS");
    compare_lib_output_33("PSX/LIB/LIBSN");
    compare_lib_output_33("PSX/LIB/LIBSND");
    compare_lib_output_33("PSX/LIB/LIBSPU");
    compare_lib_output_33("PSX/LIB/LIBTAP");
    compare_lib_output_33("PSX/SAMPLE/ETC/CARD/LIB/TURTLE");
    compare_lib_output_33("PSX/SAMPLE/ETC/CARD/LIB/SUPERX");

    compare_obj_output_33("PSX/LIB/2MBYTE");
    compare_obj_output_33("PSX/LIB/8MBYTE");
    compare_obj_output_33("PSX/LIB/MALLOC");
    compare_obj_output_33("PSX/LIB/NONE2");
    compare_obj_output_33("PSX/LIB/NONE3");
    compare_obj_output_33("PSX/SAMPLE/MODULE/EXECMENU/FONTTEX1");
}

#[test]
fn test_output_35() {
    compare_lib_output_35("PSX/LIB/LIBAPI");
    compare_lib_output_35("PSX/LIB/LIBC");
    compare_lib_output_35("PSX/LIB/LIBC2");
    compare_lib_output_35("PSX/LIB/LIBCARD");
    compare_lib_output_35("PSX/LIB/LIBCD");
    compare_lib_output_35("PSX/LIB/LIBCOMB");
    compare_lib_output_35("PSX/LIB/LIBETC");
    compare_lib_output_35("PSX/LIB/LIBGPU");
    compare_lib_output_35("PSX/LIB/LIBGS");
    compare_lib_output_35("PSX/LIB/LIBGTE");
    compare_lib_output_35("PSX/LIB/LIBGUN");
    compare_lib_output_35("PSX/LIB/LIBMATH");
    compare_lib_output_35("PSX/LIB/LIBPRESS");
    compare_lib_output_35("PSX/LIB/LIBSN");
    compare_lib_output_35("PSX/LIB/LIBSND");
    compare_lib_output_35("PSX/LIB/LIBSPU");
    compare_lib_output_35("PSX/LIB/LIBTAP");

    compare_obj_output_35("PSX/LIB/2MBYTE");
    compare_obj_output_35("PSX/LIB/8MBYTE");
    compare_obj_output_35("PSX/LIB/MALLOC");
    compare_obj_output_35("PSX/LIB/NONE2");
    compare_obj_output_35("PSX/LIB/NONE3");
}

#[test]
fn test_output_36() {
    compare_lib_output_36("PSX/LIB/LIBAPI");
    compare_lib_output_36("PSX/LIB/LIBC");
    compare_lib_output_36("PSX/LIB/LIBC2");
    compare_lib_output_36("PSX/LIB/LIBCARD");
    compare_lib_output_36("PSX/LIB/LIBCD");
    compare_lib_output_36("PSX/LIB/LIBCOMB");
    compare_lib_output_36("PSX/LIB/LIBETC");
    compare_lib_output_36("PSX/LIB/LIBGPU");
    compare_lib_output_36("PSX/LIB/LIBGS");
    compare_lib_output_36("PSX/LIB/LIBGTE");
    compare_lib_output_36("PSX/LIB/LIBGUN");
    compare_lib_output_36("PSX/LIB/LIBMATH");
    compare_lib_output_36("PSX/LIB/LIBPRESS");
    compare_lib_output_36("PSX/LIB/LIBSIO");
    compare_lib_output_36("PSX/LIB/LIBSN");
    compare_lib_output_36("PSX/LIB/LIBSND");
    compare_lib_output_36("PSX/LIB/LIBSPU");
    compare_lib_output_36("PSX/LIB/LIBTAP");
    compare_lib_output_36("PSX/SAMPLE/ETC/CARD/LIB/SUPERX");
    compare_lib_output_36("PSX/SAMPLE/ETC/CARD/LIB/TURTLE");
    compare_lib_output_36("PSX/SAMPLE/SCEE/DEMODISC/DEMO/BS/BS");
    compare_lib_output_36("PSX/SAMPLE/SCEE/DEMODISC/DEMO/NONE2/NONE2");
    compare_lib_output_36("PSX/SAMPLE/SCEE/ETC/MTAP/LIBTAP");
    compare_lib_output_36("PSX/UTILITY/MENU/CDSFILE");
    compare_lib_output_36("PSYQ/BIN/LIBDECI");
    compare_lib_output_36("PSYQ/SRC/LIBSN/LIBSN");

    compare_obj_output_36("PSX/LIB/2MBYTE");
    compare_obj_output_36("PSX/LIB/8MBYTE");
    compare_obj_output_36("PSX/LIB/NONE2");
    compare_obj_output_36("PSX/LIB/NONE3");
    compare_obj_output_36("PSX/SAMPLE/SCEE/SUBDIV/MAIN");
    compare_obj_output_36("PSX/SAMPLE/SCEE/SUBDIV/SUBDIV");
    compare_obj_output_36("PSX/UTILITY/MENU/MENU");
    compare_obj_output_36("PSX/UTILITY/MENU/PCEXEC");
    compare_obj_output_36("PSX/UTILITY/MENU/PCLOAD");
    compare_obj_output_36("PSX/UTILITY/MENU/PRINTERR");
    compare_obj_output_36("PSX/UTILITY/MENU/SDATA");
    compare_obj_output_36("PSX/UTILITY/MENU/SOUND");
    compare_obj_output_36("PSX/UTILITY/MENU/STRING");
}

#[test]
fn test_output_40() {
    compare_lib_output_40("PSX/LIB/LIBAPI");
    compare_lib_output_40("PSX/LIB/LIBC");
    compare_lib_output_40("PSX/LIB/LIBC2");
    compare_lib_output_40("PSX/LIB/LIBCARD");
    compare_lib_output_40("PSX/LIB/LIBCD");
    compare_lib_output_40("PSX/LIB/LIBCOMB");
    compare_lib_output_40("PSX/LIB/LIBDS");
    compare_lib_output_40("PSX/LIB/LIBETC");
    compare_lib_output_40("PSX/LIB/LIBGPU");
    compare_lib_output_40("PSX/LIB/LIBGS");
    compare_lib_output_40("PSX/LIB/LIBGTE");
    compare_lib_output_40("PSX/LIB/LIBGUN");
    compare_lib_output_40("PSX/LIB/LIBMATH");
    compare_lib_output_40("PSX/LIB/LIBMCRD");
    compare_lib_output_40("PSX/LIB/LIBPRESS");
    compare_lib_output_40("PSX/LIB/LIBSIO");
    compare_lib_output_40("PSX/LIB/LIBSN");
    compare_lib_output_40("PSX/LIB/LIBSND");
    compare_lib_output_40("PSX/LIB/LIBSPU");
    compare_lib_output_40("PSX/LIB/LIBTAP");
    compare_lib_output_40("PSX/SAMPLE/GRAPHICS/ZIMEN/LIBZIMEN");
    compare_lib_output_40("PSX/SAMPLE/OLD/ETC/CARD/LIB/SUPERX");
    compare_lib_output_40("PSX/SAMPLE/OLD/ETC/CARD/LIB/TURTLE");
    compare_lib_output_40("PSX/UTILITY/MENU/CDSFILE");
    compare_lib_output_40("PSYQ/PREFSMPL/LIBGS2/LIBGS");

    compare_obj_output_40("PSX/LIB/2MBYTE");
    compare_obj_output_40("PSX/LIB/8MBYTE");
    compare_obj_output_40("PSX/LIB/AUTOPAD");
    compare_obj_output_40("PSX/LIB/NOHEAP");
    compare_obj_output_40("PSX/LIB/NONE3");
    compare_obj_output_40("PSX/LIB/POWERON");
}
