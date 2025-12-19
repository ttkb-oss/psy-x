// SPDX-FileCopyrightText: © 2025 TTKB, LLC
// SPDX-License-Identifier: BSD-3-CLAUSE

//! PSY-Q Library and Object File Parser
//!
//! This crate provides parsing and manipulation capabilities for PSY-Q LIB and OBJ files,
//! which were used by the official PlayStation 1 development toolchain and third-party
//! toolchains for the Sega Saturn, Sega Genesis/MegaDrive/Sega CD/Mega CD, Super Nintendo,
//! and others.
//!
//! # Overview
//!
//! PSY-Q was the official development kit for PlayStation 1 games. It produced two main
//! types of binary files:
//!
//! - **[LIB] files**: Archive files containing multiple object modules
//! - **[OBJ] files**: Individual object files with machine code and linking information
//!
//! # Quick Start
//!
//! Reading a library file:
//!
//! ```no_run
//! use std::path::Path;
//! use psyk::io;
//! use anyhow::Result;
//!
//! fn main() -> Result<()> {
//!     let lib = io::read_lib(Path::new("LIBAPI.LIB"))?;
//!
//!     for module in lib.modules() {
//!         println!("Module: {}", module.name());
//!         println!("Created: {}", module.created());
//!         println!("Exports: {:?}", module.exports());
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! Reading either a LIB or OBJ file:
//!
//! ```no_run
//! use std::path::Path;
//! use psyk::io;
//! use anyhow::Result;
//!
//! fn main() -> Result<()> {
//!     let lib_or_obj = io::read(Path::new("SOME.OBJ"))?;
//!     println!("{}", lib_or_obj);
//!     Ok(())
//! }
//! ```

use core::cmp;
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use binrw::binrw;
use binrw::helpers::{until, until_eof};
use chrono::{
    DateTime, Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc,
};
use rabbitizer::{InstrCategory, Instruction};
use unicode_segmentation::UnicodeSegmentation;

use crate::display::DisplayWithOptions;

pub mod cli;
pub mod display;
pub mod io;
pub mod link;

/// A [LIB] is an archive of several [OBJ] files. It consists
/// of a magic number followed by one or more [Modules](Module).
///
/// A `LIB` file can be constructed from a `u8` slice using
/// `read`.
///
/// ```
/// use std::path::Path;
/// use psyk::io;
/// # use anyhow::Result;
/// # fn main() -> Result<()> {
/// let lib = io::read_lib(Path::new("SOME.LIB"));
/// # Ok(())
/// # }
/// ```
///
/// # Structure on Disk
///
/// | Offset | Type       | Description                                           |
/// |--------|------------|-------------------------------------------------------|
/// |   0    | `[u8;3]`   | Magic: "LIB"                                          |
/// |   3    | `u8`       | Archive format version (1)                            |
/// |   4    | `[Module]` | One or more [Modules](Module) which wrap [OBJ] files. |
#[binrw]
#[brw(little, magic = b"LIB", assert(!objs.is_empty()))]
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct LIB {
    version: u8,

    #[br(parse_with = until_eof)]
    objs: Vec<Module>,
}

impl LIB {
    /// Creates a new [LIB] with the provided modules.
    pub fn new(objs: Vec<Module>) -> Self {
        Self { version: 1, objs }
    }

    /// The modules contained in this library.
    ///
    /// Each module wraps an OBJ file along with metadata about its name,
    /// creation time, and exported symbols.
    pub fn modules(&self) -> &Vec<Module> {
        &self.objs
    }
}

impl fmt::Display for LIB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_with_options(f, &display::Options::default())
    }
}

impl display::DisplayWithOptions for LIB {
    fn fmt_with_options(&self, f: &mut fmt::Formatter, options: &display::Options) -> fmt::Result {
        writeln!(f, "Module     Date     Time   Externals defined")?;
        writeln!(f)?;
        for module in &self.objs {
            module.fmt_with_options(f, options)?;
            writeln!(f)?;

            if options.recursive {
                writeln!(f)?;
                module.obj.fmt_with_options(f, &options.indent())?;
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

/// An exported symbol from a module.
///
/// Exports represent functions or data that are made available to the linker
/// for use by other modules.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                                                      |
/// |--------|--------|------------------------------------------------------------------|
/// | 0      | `u8`   | Size of the symbol name.                                         |
/// | 0      | `[u8]` | The ASCII name of of the exported symbol. Not `NULL` terminated. |
#[binrw]
#[brw(little)]
#[repr(C)]
#[derive(Clone, PartialEq)]
pub struct Export {
    name_size: u8,
    #[br(count = name_size)]
    name: Vec<u8>,
}

/// An entry in the export table.
///
/// The export table is terminated by an export with a zero-length name.
impl Export {
    pub fn new(name: String) -> Self {
        // TODO: should this restrict to ascii?
        let mut utf8 = name.as_bytes().to_vec();
        utf8.truncate(u8::MAX.into());
        Self {
            name_size: name.len() as u8,
            name: utf8,
        }
    }

    pub fn empty() -> Self {
        Self {
            name_size: 0,
            name: Vec::new(),
        }
    }

    /// Returns the name of this exported symbol.
    ///
    /// Non-UTF-8 characters are replaced with the Unicode replacement character (�)
    pub fn name(&self) -> String {
        // TODO: what are * prefixed symbols for?
        if !self.name.is_empty() && self.name[0] == 0 {
            format!("*{}", String::from_utf8_lossy(&self.name[1..]).into_owned())
        } else {
            String::from_utf8_lossy(&self.name).into_owned()
        }
    }
}

/// Trait for converting PSY-Q timestamps to standard Rust date/time types.
///
/// PSY-Q uses a custom 32-bit timestamp format similar to the DOS/Windows
/// date format but with a different bit layout.
///
/// # Format
///
/// **Low 16 bits (date)**:
/// ```text
/// Bits:  15-9    8-5     4-0
///        Year    Month   Day
/// ```
/// - Year: 0-127 (relative to 1980)
/// - Month: 1-12
/// - Day: 1-31
///
/// **High 16 bits (time)**:
/// ```text
/// Bits:  15-11   10-5    4-0
///        Hour    Minute  Second/2
/// ```
/// - Hour: 0-23
/// - Minute: 0-59
/// - Second: 0-58 (stored as second/2; only even seconds)
///
/// # Note
///
/// These timestamps don't include timezone information and are treated
/// as local time in the original PSY-Q toolchain.
pub trait FromPSYQTimestamp {
    /// Converts a PSY-Q timestamp to this type.
    ///
    /// Returns `None` if the timestamp contains invalid date/time values.
    fn from_psyq_timestamp(t: u32) -> Option<Self>
    where
        Self: Sized;

    /// Converts `Self` into a 32-bit PSY-Q timestamp
    fn to_psyq_timestamp(&self) -> u32;
}

impl FromPSYQTimestamp for NaiveDate {
    fn from_psyq_timestamp(t: u32) -> Option<Self> {
        let date = t & 0xFFFF;
        let year = ((date >> 9) & 0x7F) + 1980;
        let month = (date >> 5) & 0xF;
        let day = date & 0x1F;
        NaiveDate::from_ymd_opt(year as i32, month, day)
    }

    fn to_psyq_timestamp(&self) -> u32 {
        let year = (self.year() as u32 - 1980) & 0x7F;
        let month = (self.month()) & 0xF;
        let day = (self.day()) & 0x1F;

        (year << 9) | (month << 5) | day
    }
}

impl FromPSYQTimestamp for NaiveTime {
    fn from_psyq_timestamp(t: u32) -> Option<Self> {
        let time = t >> 16;
        let hour = (time >> 11) & 0x1F;
        let minute = (time >> 5) & 0x3F;
        let second = (time & 0x1F) * 2;
        NaiveTime::from_hms_opt(hour, minute, second)
    }

    fn to_psyq_timestamp(&self) -> u32 {
        let hour = self.hour() & 0x1F;
        let minute = self.minute() & 0x3F;
        let second = self.second() / 2;

        (hour << 27) | (minute << 21) | (second << 16)
    }
}

impl FromPSYQTimestamp for NaiveDateTime {
    fn from_psyq_timestamp(t: u32) -> Option<Self> {
        // These timestamps are "local" without any timezone information.
        // We do the best we can by treating them as naive datetime values.
        Some(NaiveDateTime::new(
            NaiveDate::from_psyq_timestamp(t)?,
            NaiveTime::from_psyq_timestamp(t)?,
        ))
    }

    fn to_psyq_timestamp(&self) -> u32 {
        self.date().to_psyq_timestamp() | self.time().to_psyq_timestamp()
    }
}

impl FromPSYQTimestamp for SystemTime {
    fn from_psyq_timestamp(t: u32) -> Option<Self> {
        let dt = NaiveDateTime::from_psyq_timestamp(t)?;
        // Convert to UTC (though original timezone is unknown)
        let datetime_utc = Utc.from_utc_datetime(&dt);
        Some(UNIX_EPOCH + Duration::from_secs(datetime_utc.timestamp() as u64))
    }

    fn to_psyq_timestamp(&self) -> u32 {
        let datetime = DateTime::<Local>::from(*self);
        datetime.naive_utc().to_psyq_timestamp()
    }
}

/// Metadata for a module within a LIB archive.
///
/// This includes the module name (up to 8 characters), creation timestamp,
/// and a list of exported symbols.
///
/// # Structure on Disk
///
/// | Offset | Type       | Description                                                                          |
/// |--------|------------|--------------------------------------------------------------------------------------|
/// | 0      | `[u8; 8]`  | Module name in ASCII. Padded to 8-bytes with spaces.                                 |
/// | 8      | `u32`      | A creation timestamp for the module. The format is described in [FromPSYQTimestamp]. |
/// | 12     | `u32`      | An offset to the end of the module metadata.                                         |
/// | 16     | `u32`      | The size of the serialized [OBJ] structure.                                          |
/// | 20     | `[Export]` | An array of [Export] structs which declare any exported symbols.                     |
///
/// The list of [Export]s is terminated with a zero-length entry (a single byte with the value zero).
///
/// With the exception of the exports and offset, all other fields are derived from file system metadata. Including the
/// exports at this level allows a linker to bypass the [OBJ] if it doesn't contain any relevant symbols. The exports
/// can be retrieved by querying the [OBJ] directly using [OBJ::exports].
#[binrw]
#[brw(little)]
#[repr(C)]
#[derive(Clone, PartialEq)]
pub struct ModuleMetadata {
    name: [u8; 8],
    created: u32,
    offset: u32,
    size: u32,

    #[br(parse_with=until(|e: &Export| e.name_size == 0))]
    exports: Vec<Export>,
}

#[inline]
fn string_to_module_name(name: &str) -> [u8; 8] {
    let mut module_name: [u8; 8] = [0x20; 8];

    // the unicode path requires care to avoid breaking
    // multi-byte codepoints and grapheme clusters.
    let mut size = 0;
    for (offset, cluster) in name.grapheme_indices(false) {
        if offset > 7 || (offset + cluster.len()) > 8 {
            break;
        }
        size = offset + cluster.len();
    }

    module_name[..size].copy_from_slice(&name.as_bytes()[..size]);
    module_name
}

/// Converts a [Path] into an appropriate module name. The module
/// name is the first 8 characters of the file name without anything
/// following the first `.` (period) character (as defined by
/// [Path::file_prefix]). If that portion of the file name is smaller
/// than 8-bytes, the remaining bytes will be padded with the `NUL`
/// character.
///
/// Path does not include a file component, this function will
/// panic.
///
/// **Note on Unicode:** it is assumed that paths are encoded
/// in UTF-8, an invariant not guaranteed by the Rust std library.
/// Psy-Q was not built to handle Unicode filenames, so including
/// files with characters outside of the ASCII range will likely
/// break interoperability with other tools. However, Psy-K supports
/// Unicode file names and will produce appropriate model names
/// with only the bytes that represent full code points.
#[inline]
fn path_to_module_name(path: &Path) -> [u8; 8] {
    let Some(prefix) = path.file_prefix() else {
        panic!("Module paths must contain a file name: {:?}", path);
    };
    let binding = prefix.to_ascii_uppercase();

    if !prefix.is_ascii() {
        let Some(prefix_str) = binding.to_str() else {
            panic!("Module path is not valid unicode: {:?}", path);
        };
        return string_to_module_name(prefix_str);
    }

    // the ascii path is simple, just copy the bytes
    let bytes = binding.as_encoded_bytes();
    let mut module_name: [u8; 8] = [0x20; 8];
    let len = cmp::min(bytes.len(), module_name.len());

    module_name[0..len].copy_from_slice(&bytes[0..len]);
    module_name
}

impl ModuleMetadata {
    pub fn new(name: String, created: SystemTime, size: u32, exports: Vec<Export>) -> Self {
        let name = string_to_module_name(&name);
        let created = created.to_psyq_timestamp();
        let mut exports = exports;
        exports.push(Export::empty());

        let offset: u32 = 20 + exports.iter().map(|e| 1 + e.name_size as u32).sum::<u32>();
        Self {
            name,
            created,
            offset,
            size: offset + size,
            exports,
        }
    }

    pub fn new_from_path(path: &Path, obj: &OBJ) -> Result<Self> {
        let name = path_to_module_name(path);

        let file_metadata = fs::metadata(path)?;
        let created = if let Ok(creation_time) = file_metadata.created() {
            creation_time
        } else {
            SystemTime::now()
        };
        let exports = obj
            .exports()
            .into_iter()
            .map(Export::new)
            .collect::<Vec<Export>>();

        let size = file_metadata.len() as u32;

        Ok(Self::new(
            String::from_utf8(name.to_vec())?,
            created,
            size,
            exports,
        ))
    }

    /// Returns the module name, with trailing whitespace removed.
    ///
    /// Names will be at most 8-ASCII characters long (or 8 UTF-8 bytes).
    pub fn name(&self) -> String {
        // trim_end for the name array
        let end = self
            .name
            .iter()
            .rposition(|x| !x.is_ascii_whitespace())
            .expect("Module.name trim_end")
            + 1;
        String::from_utf8_lossy(&self.name[..end]).into_owned()
    }

    /// Returns a list of symbol names exported by this module.
    ///
    /// Empty exports (the terminator entry) are filtered out.
    pub fn exports(&self) -> Vec<String> {
        self.exports
            .iter()
            .filter_map(|e| {
                if e.name.is_empty() {
                    None
                } else {
                    Some(e.name())
                }
            })
            .collect()
    }

    /// Returns the creation timestamp as a formatted string.
    ///
    /// Format: `DD-MM-YY HH:MM:SS`
    ///
    /// # Example
    /// ```text
    /// 15-05-96 16:09:38
    /// ```
    pub fn created(&self) -> String {
        // 15-05-96 16:09:38
        //    hhhh hmmm mmms ssss yyyy yyyM MMMd dddd
        // LE 1000 0001 0011 0011 0010 0000 1010 1111
        //
        // day    - 15 01111
        // month  - 05 0101
        // year   - 96 001000
        // hour   - 16 10000
        // minute - 09 000101
        // second - 38 00010

        // format!("{} {}", self.date(), self.time())
        self.created_datetime()
            .expect("created")
            .format("%d-%m-%y %H:%M:%S")
            .to_string()
    }

    /// Returns the creation timestamp as a `NaiveDateTime`.
    ///
    /// Returns `None` if the timestamp is invalid.
    pub fn created_datetime(&self) -> Option<NaiveDateTime> {
        NaiveDateTime::from_psyq_timestamp(self.created)
    }

    /// Returns the creation timestamp as a `SystemTime`.
    ///
    /// Returns `None` if the timestamp is invalid.
    ///
    /// Note: The original timestamp has no timezone information, so it's
    /// treated as UTC for conversion purposes.
    pub fn created_at(&self) -> Option<SystemTime> {
        SystemTime::from_psyq_timestamp(self.created)
    }
}

/// A module entry in a LIB archive.
///
/// Each module consists of metadata (name, timestamp, exports) and the
/// actual OBJ file data.
///
/// # Structure on Disk
///
/// | Offset          | Type             | Description                                                                |
/// |-----------------|------------------|----------------------------------------------------------------------------|
/// | 0               | `ModuleMetadata` | [ModuleMetadata] containing the name, exports, and additional information. |
/// | sizeof(*metadata*) | `OBJ`         | An [OBJ] as it appears on disk.                                            |
///
/// A `Module` contains a parsed and hydrated [OBJ]. If only the metadata is of interest, [OpaqueModule] can be used
/// instead.
#[binrw]
#[brw(little)]
#[repr(C)]
#[derive(Clone, PartialEq)]
pub struct Module {
    metadata: ModuleMetadata,
    obj: OBJ,
}

impl Module {
    /// Create a new [Module] programmatically
    pub fn new(obj: OBJ, metadata: ModuleMetadata) -> Self {
        Self { metadata, obj }
    }

    /// Creates a new [Module] from the file at `path`.
    ///
    /// `path` must point to a valid [OBJ] file.
    pub fn new_from_path(path: &Path) -> Result<Self> {
        let obj = io::read_obj(path)?;
        let metadata = ModuleMetadata::new_from_path(path, &obj)?;
        Ok(Self { metadata, obj })
    }

    /// Returns the module name.
    pub fn name(&self) -> String {
        self.metadata.name()
    }

    /// Returns the list of exported symbol names.
    pub fn exports(&self) -> Vec<String> {
        self.metadata.exports()
    }

    /// Returns the creation timestamp as a formatted string.
    pub fn created(&self) -> String {
        self.metadata.created()
    }

    /// Returns the creation timestamp as a `SystemTime`
    pub fn created_at(&self) -> Option<SystemTime> {
        self.metadata.created_at()
    }

    /// Returns the creation timestamp as a `NaiveDateTime`
    pub fn created_datetime(&self) -> Option<NaiveDateTime> {
        self.metadata.created_datetime()
    }

    /// Returns a reference to the OBJ file contained in this module.
    pub fn object(&self) -> &OBJ {
        &self.obj
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_with_options(f, &display::Options::default())
    }
}

impl display::DisplayWithOptions for Module {
    fn fmt_with_options(&self, f: &mut fmt::Formatter, _options: &display::Options) -> fmt::Result {
        write!(
            f,
            "{:<8} {} {}",
            self.name(),
            self.created(),
            self.exports()
                .into_iter()
                .map(|e| format!("{e} "))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}

impl fmt::Debug for Module {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Module {{name: \"{}\", huh: {}, offset: {}, size: {}, exports: \"{:?}\", lnk: {:?}}}",
            self.name(),
            self.metadata.created,
            self.metadata.offset,
            self.metadata.size,
            self.exports(),
            self.obj
        )
    }
}

/// An opaque module representation used during parsing.
///
/// This variant stores the raw bytes of the OBJ file without parsing it,
/// which can be useful for tools that only need to inspect metadata.
///
/// # Structure on Disk
///
/// | Offset             | Type             | Description                                                                |
/// |--------------------|------------------|----------------------------------------------------------------------------|
/// | 0                  | `ModuleMetadata` | [ModuleMetadata] containing the name, exports, and additional information. |
/// | sizeof(*metadata*) | `[u8]`           | A binary blob containing a serialized [OBJ].                               |
///
/// If a parsed and hydrated [OBJ] is needed, use [Module] instead.
#[binrw]
#[brw(little)]
#[repr(C)]
pub struct OpaqueModule {
    metadata: ModuleMetadata,

    #[br(count = metadata.size - 16)]
    obj: Vec<u8>,
}

impl OpaqueModule {
    /// Returns the module name.
    pub fn name(&self) -> String {
        self.metadata.name()
    }

    /// Returns the list of exported symbol names.
    pub fn exports(&self) -> Vec<String> {
        self.metadata.exports()
    }

    /// Returns the creation timestamp as a formatted string.
    pub fn created(&self) -> String {
        self.metadata.created()
    }

    /// Returns the creation timestamp as a `SystemTime`.
    pub fn created_at(&self) -> Option<SystemTime> {
        self.metadata.created_at()
    }

    /// Returns the creation timestamp as a `NaiveDateTime`.
    pub fn created_datetime(&self) -> Option<NaiveDateTime> {
        self.metadata.created_datetime()
    }

    /// Returns a reference to the OBJ binary data.
    pub fn obj_blob(&self) -> &[u8] {
        &self.obj
    }
}

/// A PSY-Q object file (LNK format).
///
/// OBJ files contain machine code, relocation information, symbol definitions,
/// and debugging data needed by the linker.
///
/// # Structure on Disk
///
/// | Offset | Type        | Description               |
/// |--------|-------------|---------------------------|
/// | 0      | `[u8; 3]`   | Magic number: "LNK"       |
/// | 3      | `u8`        | Version (typically 2)     |
/// | 4      | `[Section]` | [Section]s until `NOP`    |
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use psyk::io;
/// use anyhow::Result;
///
/// fn main() -> Result<()> {
///     let obj = io::read_obj(Path::new("MODULE.OBJ"))?;
///
///     println!("OBJ version: {}", obj.version());
///     println!("Sections: {}", obj.sections().len());
///
///     Ok(())
/// }
/// ```
#[binrw]
#[brw(little, magic = b"LNK")]
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct OBJ {
    version: u8,
    #[br(parse_with=until(|section: &Section| matches!(section, Section::NOP)))]
    sections: Vec<Section>,
}

impl OBJ {
    pub fn new(sections: Vec<Section>) -> Self {
        assert!(matches!(sections.last(), Some(Section::NOP)));
        Self {
            version: 2,
            sections,
        }
    }

    /// Returns the OBJ format version (typically 2).
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Returns the sections contained in this object file.
    ///
    /// Sections include code, data, symbols, relocations, and debug info.
    /// The list is terminated by a `Section::NOP` entry.
    pub fn sections(&self) -> &Vec<Section> {
        &self.sections
    }

    /// Returns symbols exported by this object file.
    ///
    /// Exported symbols can be functions or globals.
    pub fn exports(&self) -> Vec<String> {
        self.sections()
            .iter()
            .filter_map({
                |s| match s {
                    Section::XDEF(xdef) => Some(xdef.symbol_name()),
                    Section::XBSS(xbss) => Some(xbss.name()),
                    _ => None,
                }
            })
            .collect()
    }
}

impl fmt::Display for OBJ {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_with_options(f, &display::Options::default())
    }
}

impl display::DisplayWithOptions for OBJ {
    fn fmt_with_options(&self, f: &mut fmt::Formatter, options: &display::Options) -> fmt::Result {
        options.write_indent(f)?;
        writeln!(f, "Header : LNK version {}", self.version)?;
        for section in &self.sections {
            section.fmt_with_options(f, options)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

/// Machine code section.
///
/// Contains executable instructions for the target [CPU](Section::CPU).
///
/// # Structure on Disk
///
/// | Offset | Type   | Description       |
/// |--------|--------|-------------------|
/// | 0      | `u16`  | Size of the code. |
/// | 1      | `[u8]` | Machine code.     |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct Code {
    size: u16,
    #[br(count = size)]
    code: Vec<u8>,
}

impl Code {
    /// Returns the code for this section as bytes. Their format can be determined by the value
    /// set in the [CPU](Section::CPU).
    pub fn code(&self) -> &Vec<u8> {
        &self.code
    }
}

/// An expression used in relocations.
///
/// PSY-Q uses a sophisticated expression system for calculating relocated
/// addresses. Expressions can be constants, symbol references, or complex
/// arithmetic operations.
///
/// Each component of an expression has a unique on disk format. There are
/// no synchronization points, or sizes encoded into the structure, so it
/// is important that each type is explicitly modeled.
///
/// Linker expressions are similar, but not identical to assembler expressions
/// and some built-in functions are available in both.
///
/// # Example Expressions
///
/// - `$1000` - Constant value 0x1000
/// - `[5]` - Address of symbol #5
/// - `sectbase(2)` - Base address of section #2
/// - `(sectstart(1)+$100)` - Section 1 start plus 0x100
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    /// A constant value.
    ///
    /// ```asm
    /// $123D
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x0  |
    /// | 1      | `u32`  | Value.      |
    #[brw(magic(0u8))]
    Constant(u32),

    /// Index of a symbol's address.
    ///
    /// ```asm
    /// [x]
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x2  |
    /// | 1      | `u16`  | ???         |
    #[brw(magic(2u8))]
    SymbolAddressIndex(u16),

    /// Base address of a section.
    ///
    /// ```asm
    /// sectbase(x)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x4  |
    /// | 1      | `u16`  | ???         |
    #[brw(magic(4u8))]
    SectionAddressIndex(u16),

    /// Untested
    ///
    /// ```asm
    /// bank(x)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x6  |
    /// | 1      | `u16`  | ???         |
    #[brw(magic(6u8))]
    Bank(u16),

    /// Untested
    ///
    /// ```asm
    /// sectof(x)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x8  |
    /// | 1      | `u16`  | Section ID. |
    #[brw(magic(8u8))]
    SectionOffset(u16),

    /// Untested
    ///
    /// ```asm
    /// offs(x)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0xA  |
    /// | 1      | `u16`  | ???         |
    // 10 - offs({})
    #[brw(magic(10u8))]
    Offset(u16),

    /// Start address of a section.
    ///
    /// ```asm
    /// sectstart(x)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0xC  |
    /// | 1      | `u16`  | Section ID. |
    #[brw(magic(12u8))]
    SectionStart(u16),

    /// Untested
    ///
    /// ```asm
    /// groupstart(x)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0xE  |
    /// | 1      | `u16`  | Group ID.   |
    #[brw(magic(14u8))]
    GroupStart(u16),

    /// The offset of a group.
    ///
    /// Untested
    ///
    /// ```asm
    /// groupof(x)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x10 |
    /// | 1      | `u16`  | Group ID.   |
    #[brw(magic(16u8))]
    GroupOffset(u16),

    /// Untested
    ///
    /// ```asm
    /// seg(x)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x12 |
    /// | 1      | `u16`  | ???         |
    #[brw(magic(18u8))]
    Segment(u16),

    /// The `ORG` address of a group for a symbol.
    ///
    /// ```asm
    /// grouporg(x)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x14 |
    /// | 1      | `u16`  | Symbol ID.  |
    #[brw(magic(20u8))]
    GroupOrg(u16),

    /// End address of a section.
    ///
    /// ```asm
    /// sectend(X)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type  | Description  |
    /// |--------|-------|--------------|
    /// | 0      | `u8`  | Magic: 0x16  |
    /// | 1      | `u16` | Section ID.  |
    #[brw(magic(22u8))]
    SectionEnd(u16),

    //
    // Comparison operators
    //
    /// Equality comparison.
    ///
    /// ```asm
    /// (a=b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset                        | Type         | Description         |
    /// |-------------------------------|--------------|---------------------|
    /// | 0                             | `u8`         | Magic: 0x20         |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(32u8))]
    Equals(Box<Expression>, Box<Expression>),

    /// Inequality comparison.
    ///
    /// ```asm
    /// (a<>b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0                             | `u8`   | Magic: 0x22  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(34u8))]
    NotEquals(Box<Expression>, Box<Expression>),

    /// Less than or equal.
    ///
    /// ```asm
    /// (a<=b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x24  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(36u8))]
    LTE(Box<Expression>, Box<Expression>),

    /// Less than.
    ///
    /// ```asm
    /// (a<b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x26  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(38u8))]
    LessThan(Box<Expression>, Box<Expression>),

    /// Greater than or equal.
    ///
    /// ```asm
    /// (a>=b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x28  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(40u8))]
    GTE(Box<Expression>, Box<Expression>),

    /// Greater than.
    ///
    /// ```asm
    /// (a>b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x2A  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(42u8))]
    GreaterThan(Box<Expression>, Box<Expression>),

    //
    // Arithmetic operators
    //
    /// Addition.
    ///
    /// ```asm
    /// (a+b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x2C  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(44u8))]
    Add(Box<Expression>, Box<Expression>),

    /// Subtraction.
    ///
    /// ```asm
    /// (a-b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x2E |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(46u8))]
    Subtract(Box<Expression>, Box<Expression>),

    /// Multiplication.
    ///
    /// ```asm
    /// (a*b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x30  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(48u8))]
    Multiply(Box<Expression>, Box<Expression>),

    /// Division.
    ///
    /// ```asm
    /// (a/b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x32  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(50u8))]
    Divide(Box<Expression>, Box<Expression>),

    /// Bitwise AND.
    ///
    /// ```asm
    /// (a&b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x34  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(52u8))]
    And(Box<Expression>, Box<Expression>),

    /// Bitwise OR operator.
    ///
    /// ```asm
    /// (a!b)
    /// ```
    ///
    /// Instead of using the typical `|` (pipe) symbol, the default rendering
    /// of this operator is the `!` (exclamation point/bang) symbol. In the
    /// assembler, the `|` symbol acts as an alias for `!`.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x36  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(54u8))]
    Or(Box<Expression>, Box<Expression>),

    /// Bitwise XOR.
    ///
    /// ```asm
    /// (a^b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x38  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(56u8))]
    XOR(Box<Expression>, Box<Expression>),

    /// Left shift.
    ///
    /// ```asm
    /// (a<<b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x3A  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(58u8))]
    LeftShift(Box<Expression>, Box<Expression>),

    /// Right shift.
    ///
    /// ```asm
    /// (a>>b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x3C  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(60u8))]
    RightShift(Box<Expression>, Box<Expression>),

    /// Modulo.
    ///
    /// ```asm
    /// (a%b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x3E  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(62u8))]
    Mod(Box<Expression>, Box<Expression>),

    /// Dashes operator.
    ///
    /// ```asm
    /// (a---b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset                        | Type         | Description         |
    /// |-------------------------------|--------------|---------------------|
    /// | 0                             | `u8`         | Magic: 0x40         |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(64u8))]
    Dashes(Box<Expression>, Box<Expression>),

    //
    // Special operators (primarily for Saturn/SH-2)
    //
    /// Reverse word.
    ///
    /// ```asm
    /// (a-revword-b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x42  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(66u8))]
    Revword(Box<Expression>, Box<Expression>),

    /// Check0.
    ///
    /// ```asm
    /// (a-check0-b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description |
    /// |--------|--------|-------------|
    /// | 0      | `u8`   | Magic: 0x44  |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(68u8))]
    Check0(Box<Expression>, Box<Expression>),

    /// Check1.
    ///
    /// ```asm
    /// (a-check1-b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset                        | Type         | Description         |
    /// |-------------------------------|--------------|---------------------|
    /// | 0                             | `u8`         | Magic: 0x46         |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(70u8))]
    Check1(Box<Expression>, Box<Expression>),

    /// Bit range extraction.
    ///
    /// ```asm
    /// (a-bitrange-b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset                        | Type         | Description         |
    /// |-------------------------------|--------------|---------------------|
    /// | 0                             | `u8`         | Magic: 0x48         |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(72u8))]
    BitRange(Box<Expression>, Box<Expression>),

    /// Arithmetic shift with check.
    ///
    /// ```asm
    /// (a-arshift_chk-b)
    /// ```
    ///
    /// # Structure on Disk
    ///
    /// | Offset                        | Type         | Description         |
    /// |-------------------------------|--------------|---------------------|
    /// | 0                             | `u8`         | Magic: 0x4A         |
    /// | 1                             | `Expression` | Left [Expression].  |
    /// | sizeof(*left_expression*) + 1 | `Expression` | Right [Expression]. |
    #[brw(magic(74u8))]
    ArshiftChk(Box<Expression>, Box<Expression>),
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Constant(value) => write!(f, "${value:x}"),
            Self::SymbolAddressIndex(addr) => write!(f, "[{addr:x}]"),
            Self::SectionAddressIndex(base) => write!(f, "sectbase({base:x})"),
            // untested
            Self::Bank(bank) => write!(f, "bank({bank:x})"),
            // untested
            Self::SectionOffset(section) => write!(f, "sectof({section:x})"),
            // untested
            Self::Offset(bank) => write!(f, "offs({bank:x})"),
            Self::SectionStart(offset) => write!(f, "sectstart({offset:x})"),
            // untested
            Self::GroupStart(group) => write!(f, "groupstart({group:x})"),
            // untested
            Self::GroupOffset(group) => write!(f, "groupof({group:x})"),
            // untested
            Self::Segment(segment) => write!(f, "seg({segment:x})"),
            // untested
            Self::GroupOrg(group) => write!(f, "grouporg({group:x})"),
            Self::SectionEnd(offset) => write!(f, "sectend({offset:x})"),

            // comparison
            Self::Equals(lhs, rhs) => write!(f, "({lhs}={rhs})"),
            Self::NotEquals(lhs, rhs) => write!(f, "({lhs}<>{rhs})"),
            Self::LTE(lhs, rhs) => write!(f, "({lhs}<={rhs})"),
            Self::LessThan(lhs, rhs) => write!(f, "({lhs}<{rhs})"),
            Self::GTE(lhs, rhs) => write!(f, "({lhs}>={rhs})"),
            Self::GreaterThan(lhs, rhs) => write!(f, "({lhs}>{rhs})"),

            // arithmatic
            Self::Add(lhs, rhs) => write!(f, "({lhs}+{rhs})"),
            Self::Subtract(lhs, rhs) => write!(f, "({lhs}-{rhs})"),
            Self::Multiply(lhs, rhs) => write!(f, "({lhs}*{rhs})"),
            Self::Divide(lhs, rhs) => write!(f, "({lhs}/{rhs})",),
            Self::And(lhs, rhs) => write!(f, "({lhs}&{rhs})"),
            Self::Or(lhs, rhs) => write!(f, "({lhs}!{rhs})"),
            Self::XOR(lhs, rhs) => write!(f, "({lhs}^{rhs})"),
            Self::LeftShift(lhs, rhs) => write!(f, "({lhs}<<{rhs})"),
            Self::RightShift(lhs, rhs) => write!(f, "({lhs}>>{rhs})"),
            Self::Mod(lhs, rhs) => write!(f, "({lhs}%%{rhs})"),
            Self::Dashes(lhs, rhs) => write!(f, "({lhs}---{rhs})"),

            // keyword
            Self::Revword(lhs, rhs) => write!(f, "({lhs}-revword-{rhs})"),
            Self::Check0(lhs, rhs) => write!(f, "({lhs}-check0-{rhs})"),
            Self::Check1(lhs, rhs) => write!(f, "({lhs}-check1-{rhs})"),
            Self::BitRange(lhs, rhs) => write!(f, "({lhs}-bitrange-{rhs})"),
            Self::ArshiftChk(lhs, rhs) => write!(f, "({lhs}-arshift_chk-{rhs})"),
        }
    }
}

/// A relocation patch to be applied by the linker.
///
/// Patches modify code or data at a specific offset using a calculated expression.
///
/// # Types
///
/// | Tag | Description                                              | Expression             |
/// |-----|----------------------------------------------------------|------------------------|
/// | 8   | Write 32-bit expression value (big-endian?)              | ``                     |
/// | 10  | Unknown                                                  |                        |
/// | 16  | Write 32-bit expression value (little-endian?)           | ``                     |
/// | 30  | Possibly related to register allocation.                 |                        |
/// | 74  | Function symbol relocation (24-bit, little-endian).      | `[14]`                 |
/// | 82  | Copy expression high 16-bytes into instruction low bytes | `($20+sectbase(f001))` |
/// | 84  | Copy expression low 16-bytes into instruction low bytes  | `($20+sectbase(f001))` |
///
/// # Structure on Disk
///
/// | Offset | Type         | Description                                              |
/// |--------|--------------|----------------------------------------------------------|
/// | 0      | `u8`         | The type of patch.                                       |
/// | 1      | `u16`        | Offset in the section where the patch should be applied. |
/// | 3      | `Expression` | An [Expression] to use calculate the patch value.        |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    /// The type of patch (determines how the expression value is applied).
    tag: u8,
    /// Offset in the current section where the patch should be applied.
    offset: u16,
    /// Expression to calculate the patch value.
    expression: Expression,
}

/// Section header information.
///
/// Defines properties of a section such as its group, alignment, and type name.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                                          |
/// |--------|--------|------------------------------------------------------|
/// | 0      | `u16`  | Section ID.                                          |
/// | 2      | `u16`  | Group ID.                                            |
/// | 4      | `u8`   | Alignemnt.                                           |
/// | 5      | `u8`   | Size of the `type_name` string.                      |
/// | 6      | `[u8]` | The name of the section type. Not `NULL` terminated. |
#[binrw]
#[brw(little)]
#[derive(Clone, PartialEq)]
pub struct LNKHeader {
    section: u16,
    group: u16,
    align: u8,
    type_name_size: u8,

    #[br(count = type_name_size)]
    type_name: Vec<u8>,
}

impl LNKHeader {
    /// Returns the section type name (e.g., ".text", ".data", ".bss").
    pub fn type_name(&self) -> String {
        String::from_utf8_lossy(&self.type_name).into_owned()
    }
}

impl fmt::Debug for LNKHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LNKHeader {{section: {}, group: {}, align: {}, type_name: \"{}\"}}",
            self.section,
            self.group,
            self.align,
            self.type_name(),
        )
    }
}

/// A local symbol definition.
///
/// Local symbols are visible only within the current module.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                             |
/// |--------|--------|-----------------------------------------|
/// | 0      | `u16`  | Section ID.                             |
/// | 2      | `u32`  | Offset (TODO: relative to what?)        |
/// | 6      | `u8`   | Size of the symbol name string.         |
/// | 7      | `[u8]` | The symbol name. Not `NULL` terminated. |
#[binrw]
#[brw(little)]
#[derive(Clone, PartialEq)]
pub struct LocalSymbol {
    section: u16,
    offset: u32,
    name_size: u8,

    #[br(count = name_size)]
    name: Vec<u8>,
}

impl LocalSymbol {
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }
}

impl fmt::Debug for LocalSymbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LocalSymbol {{section: {}, offset: {}, name: \"{}\"}}",
            self.section,
            self.offset,
            self.name(),
        )
    }
}

/// A group symbol definition.
///
/// Groups are used to organize sections for linking.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                             |
/// |--------|--------|-----------------------------------------|
/// | 0      | `u16`  | Number.                                 |
/// | 2      | `u8`   | Symbol type.                            |
/// | 6      | `u8`   | Size of the symbol name string.         |
/// | 7      | `[u8]` | The symbol name. Not `NULL` terminated. |
#[binrw]
#[brw(little)]
#[derive(Clone, PartialEq)]
pub struct GroupSymbol {
    number: u16,
    sym_type: u8,
    name_size: u8,

    #[br(count = name_size)]
    name: Vec<u8>,
}

impl GroupSymbol {
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }
}

impl fmt::Debug for GroupSymbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "GroupSymbol {{number: {}, type: {}, name: \"{}\"}}",
            self.number,
            self.sym_type,
            self.name(),
        )
    }
}

/// An external symbol definition (XDEF).
///
/// XDEF entries define symbols that are exported from this module
/// and can be referenced by other modules.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                             |
/// |--------|--------|-----------------------------------------|
/// | 0      | `u16`  | Number.                                 |
/// | 2      | `u16`  | Section ID.                             |
/// | 4      | `u32`  | Offset (TODO: relative to what?)        |
/// | 8      | `u8`   | Size of the symbol name string.         |
/// | 9      | `[u8]` | The symbol name. Not `NULL` terminated. |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct XDEF {
    number: u16,
    section: u16,
    offset: u32,
    symbol_name_size: u8,

    #[br(count = symbol_name_size)]
    symbol_name: Vec<u8>,
}

impl XDEF {
    pub fn symbol_name(&self) -> String {
        // TODO: can a starred symbol be here as well?
        String::from_utf8_lossy(&self.symbol_name).into_owned()
    }
}

/// An external symbol reference (XREF).
///
/// XREF entries declare symbols that this module needs but are
/// defined in other modules.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                             |
/// |--------|--------|-----------------------------------------|
/// | 0      | `u16`  | Number.                                 |
/// | 2      | `u8`   | Size of the symbol name string.         |
/// | 3      | `[u8]` | The symbol name. Not `NULL` terminated. |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct XREF {
    number: u16,
    symbol_name_size: u8,

    #[br(count = symbol_name_size)]
    symbol_name: Vec<u8>,
}

impl XREF {
    pub fn symbol_name(&self) -> String {
        String::from_utf8_lossy(&self.symbol_name).into_owned()
    }
}

/// A file name reference used in debug information.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                                  |
/// |--------|--------|----------------------------------------------|
/// | 0      | `u16`  | The file number.                             |
/// | 2      | `u8`   | The size of the file name.                   |
/// | 3      | `[u8]` | The name of the file. Not `NULL` terminated. |
#[binrw]
#[brw(little)]
#[derive(Clone, PartialEq)]
pub struct Filename {
    number: u16,
    size: u8,
    #[br(count = size)]
    name: Vec<u8>,
}

impl Filename {
    pub fn number(&self) -> u16 {
        self.number
    }

    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }
}

impl fmt::Debug for Filename {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Filename {{number: {}, name: \"{}\"}}",
            self.number,
            self.name(),
        )
    }
}

/// Set MX info directive.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                                  |
/// |--------|--------|----------------------------------------------|
/// | 0      | `u16`  | Offset (TODO: relative to what?)             |
/// | 2      | `u8`   | Value.                                       |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct SetMXInfo {
    offset: u16,
    value: u8,
}

/// External BSS (uninitialized data) symbol.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                                    |
/// |--------|--------|------------------------------------------------|
/// | 0      | `u16`  | Number.                                        |
/// | 2      | `u16`  | The section ID this BSS belongs to.            |
/// | 4      | `u32`  | The size of the uninitialized data.            |
/// | 8      | `u8`   | The size of the symbol name.                   |
/// | 9      | `[u8]` | The name of the symbol. Not `NULL` terminated. |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct XBSS {
    number: u16,
    section: u16,
    size: u32,
    name_size: u8,

    #[br(count = name_size)]
    name: Vec<u8>,
}

impl XBSS {
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }
}

/// Set source line debugger (SLD) line number.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                      |
/// |--------|--------|----------------------------------|
/// | 0      | `u16`  | Offset (TODO: relative to what?) |
/// | 2      | `u32`  | Line number.                     |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct SetSLDLineNum {
    offset: u16,
    linenum: u32,
}

/// Set source line debugger (SLD) line number with file reference.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                       |
/// |--------|--------|-----------------------------------|
/// | 0      | `u16`  | Offset (TODO: relative to what?). |
/// | 2      | `u32`  | Line number.                      |
/// | 6      | `u16`  | File ID.                          |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct SetSLDLineNumFile {
    offset: u16,
    linenum: u32,
    file: u16,
}

/// **n.b.!** this is completely untested and based on
/// assumptions from the output from `dumpobj`.
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct ProcedureCall {
    distance: u8,
    symbol: u16,
}

/// **n.b.!** this is completely untested and based on
/// assumptions from the output from `dumpobj`.
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct ProcedureDefinition {
    symbol: u16,
}

/// Function start debug information.
///
/// Provides detailed information about a function for source-level debugging.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                                      |
/// |--------|--------|--------------------------------------------------|
/// | 0      | `u16`  | Section ID.                                      |
/// | 2      | `u32`  | Offset (TODO: relative to what?)                 |
/// | 6      | `u16`  | File ID.                                         |
/// | 8      | `u32`  | Line number.                                     |
/// | 12     | `u16`  | Frame register.                                  |
/// | 14     | `u32`  | Frame size.                                      |
/// | 18     | `u16`  | Return PC register.                              |
/// | 20     | `u32`  | Mask.                                            |
/// | 24     | `i32`  | Mask offset. (TODO: relative to what?)           |
/// | 28     | `u8`   | The size of the function name.                   |
/// | 29     | `[u8]` | The name of the function. Not `NULL` terminated. |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionStart {
    section: u16,
    offset: u32,
    file: u16,
    linenum: u32,
    frame_register: u16,
    frame_size: u32,
    return_pc_register: u16,
    mask: u32,
    mask_offset: i32,

    name_size: u8,
    #[br(count = name_size)]
    name: Vec<u8>,
}

impl FunctionStart {
    /// Function end debug information.
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }
}

/// Section, Offset, and Line number information for source-line debugging.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                       |
/// |--------|--------|-----------------------------------|
/// | 0      | `u16`  | Section ID.                       |
/// | 2      | `u32`  | Offset (TODO: relative to what?   |
/// | 6      | `u32`  | Line number.                      |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct SectionOffsetLine {
    section: u16,
    offset: u32,
    linenum: u32,
}

/// Variable or type definition debug information.
///
/// # Structure on Disk
///
/// | Offset | Type   | Description                                    |
/// |--------|--------|------------------------------------------------|
/// | 0      | `u16`  | Section ID.                                    |
/// | 2      | `u32`  | Value.                                         |
/// | 6      | `u16`  | Class ID.                                      |
/// | 8      | `u16`  | Definition type.                               |
/// | 10     | `u32`  | Data size.                                     |
/// | 14     | `u8`   | The size of the symbol name.                   |
/// | 15     | `[u8]` | The name of the symbol. Not `NULL` terminated. |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct Def {
    section: u16,
    value: u32,
    class: u16,
    def_type: u16,
    size: u32,
    name_size: u8,
    #[br(count = name_size)]
    name: Vec<u8>,
}

impl Def {
    /// Returns the definition name.
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }
}

/// Dimension specification for arrays.
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub enum Dim {
    /// No dimensions (scalar).
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type | Description |
    /// |--------|------|-------------|
    /// | 0      | `u8` | Magic: 0x0  |
    #[br(magic = 0u16)]
    None,

    /// Single dimension with size.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type  | Description |
    /// |--------|-------|-------------|
    /// | 0      | `u16` | Magic: 0x1  |
    /// | 1      | `u32` | Size        |
    #[br(magic = 1u16)]
    Value(u32),
}

impl fmt::Display for Dim {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::None => write!(f, "0"),
            Self::Value(v) => write!(f, "1 {v}"),
        }
    }
}

/// Extended variable or type definition with additional metadata.
///
/// # Structure on Disk
///
/// | Offset                       | Type   | Description                                    |
/// |------------------------------|--------|------------------------------------------------|
/// | 0                            | `u16`  | Section ID.                                    |
/// | 2                            | `u32`  | Value.                                         |
/// | 6                            | `u16`  | Class.                                         |
/// | 8                            | `u16`  | Definition type.                               |
/// | 10                           | `u32`  | Size of the data.                              |
/// | 14                           | `Dim`  | A [Dim] describing the dimensions              |
/// | sizeof(*dim*) + 14           | `u8`   | Size of the tag string.                        |
/// | offsetof(*tag_size*) + 1     | `[u8]` | Tag name string. Not `NULL` terminated.        |
/// | offsetof(*tag*) + *tag_size* | `u8`   | Size of the name string.                       |
/// | offsetof(*name_size*) + 1    | `[u8]` | Definition name string. Not `NULL` terminated. |
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub struct Def2 {
    section: u16,
    value: u32,
    class: u16,
    def_type: u16, // 34 00
    size: u32,     // 04 00 00 00
    dims: Dim,
    tag_size: u8,
    #[br(count = tag_size)]
    tag: Vec<u8>,
    name_size: u8, // 06
    #[br(count = name_size)]
    name: Vec<u8>, // 75 5F 63 68 61 72
}

impl Def2 {
    pub fn tag(&self) -> String {
        String::from_utf8_lossy(&self.tag).into_owned()
    }

    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }
}

pub mod cputype {
    //! CPU architecture type identifiers.
    //!
    //! [MOTOROLA_68000], [MIPS_R3000], and [HITACHI_SH2] have been found
    //! in the wild. Others are speculative based on available information.

    /// Motorola 68000 - Sega Genesis, Sega CD, Mega Drive, & Mega CD.
    pub const MOTOROLA_68000: u8 = 0;

    /// Motorola 68010.
    pub const MOTOROLA_68010: u8 = 1;

    /// Motorola 68020.
    pub const MOTOROLA_68020: u8 = 2;

    /// Motorola 68030.
    pub const MOTOROLA_68030: u8 = 3;

    /// Motorola 68040.
    pub const MOTOROLA_68040: u8 = 4;

    /// WDC 65816 - Used for SNES derivative Ricoh 5A22.
    pub const WDC_65816: u8 = 5;

    /// Zilog Z80 - Sega Genesis Sound CPU.
    pub const ZILOG_Z80: u8 = 6;

    /// MIPS R3000 with GTE (Graphics Transform Engine) - PlayStation 1.
    pub const MIPS_R3000: u8 = 7;

    /// Hitachi SH-2 - Sega Saturn.
    pub const HITACHI_SH2: u8 = 8;
}

fn unimplemented(s: &str) -> bool {
    eprintln!("Unimplemented: {s}");
    false
}

/// A section within an OBJ file.
///
/// Sections can contain code, data, relocations, symbols, or debug information.
/// The section list is terminated by a NOP entry.
///
/// # Section Types
///
/// - [Code](Section::Code): Executable machine code
/// - [BSS](Section::BSS): Uninitialized data
/// - [XDEF](Section::XDEF)/[XREF](Section::XREF): Symbol exports and imports
/// - [Patch](Section::Patch): Relocation information
/// - Debug information: Line numbers, function info, etc.
/// - And many more!
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq)]
pub enum Section {
    /// End of file marker.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type | Description               |
    /// |--------|------|---------------------------|
    /// | 0      | `u8` | Magic: 0x0                |
    #[brw(magic(0u8))]
    NOP,

    /// Machine code.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description        |
    /// |--------|--------|--------------------|
    /// | 0      | `u8`   | Magic: 0x2         |
    /// | 1      | `Code` | A [Code] structure |
    #[brw(magic(2u8))]
    Code(Code),

    /// Run at offset.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type  | Description       |
    /// |--------|-------|-------------------|
    /// | 0      | `u8`  | Magic: 0x4        |
    /// | 1      | `u16` |                   |
    /// | 3      | `u16` |                   |
    #[brw(magic(4u8))]
    RunAtOffset(u16, u16),

    /// Switch to different section.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type  | Description       |
    /// |--------|-------|-------------------|
    /// | 0      | `u8`  | Magic: 0x2        |
    /// | 1      | `u16` | Section ID        |
    #[brw(magic(6u8))]
    SectionSwitch(u16),

    /// Uninitialized data (BSS) with size in bytes.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description       |
    /// |--------|--------|-------------------|
    /// | 0      | `u8`   | Magic: 0x8        |
    /// | 1      | `u32`  | Size in bytes     |
    #[brw(magic(8u8))]
    BSS(u32),

    /// Relocation patch.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type    | Description         |
    /// |--------|---------|---------------------|
    /// | 0      | `u8`    | Magic: 0xA          |
    /// | 1      | `Patch` | A [Patch] structure |
    #[brw(magic(10u8))]
    Patch(Patch),

    /// External symbol definition.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description          |
    /// |--------|--------|----------------------|
    /// | 0      | `u8`   | Magic: 0xC           |
    /// | 1      | `XDEF` | An [XDEF] structure. |
    #[brw(magic(12u8))]
    XDEF(XDEF),

    /// External symbol reference.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description          |
    /// |--------|--------|----------------------|
    /// | 0      | `u8`   | Magic: 0xE           |
    /// | 1      | `XREF` | An [XREF] structure. |
    #[brw(magic(14u8))]
    XREF(XREF),

    /// Section header.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type        | Description               |
    /// |--------|-------------|---------------------------|
    /// | 0      | `u8`        | Magic: 0x10               |
    /// | 1      | `LNKHeader` | An [LNKHeader] structure. |
    #[brw(magic(16u8))]
    LNKHeader(LNKHeader),

    /// Local symbol.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type          | Description               |
    /// |--------|---------------|---------------------------|
    /// | 0      | `u8`          | Magic: 0x12               |
    /// | 1      | `LocalSymbol` | A [LocalSymbol] structure |
    #[brw(magic(18u8))]
    LocalSymbol(LocalSymbol),

    /// Group symbol.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type          | Description                |
    /// |--------|---------------|----------------------------|
    /// | 0      | `u8`          | Magic: 0x14                |
    /// | 1      | `GroupSymbol` | A [GroupSymbol] structure. |
    #[brw(magic(20u8))]
    GroupSymbol(GroupSymbol),

    /// Untested
    #[brw(magic(22u8))]
    ByteSizeRegister(u16),

    /// Untested
    #[brw(magic(24u8))]
    WordSizeRegister(u16),

    /// Untested
    #[brw(magic(26u8))]
    LongSizeRegister(u16),

    /// File name reference.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type       | Description             |
    /// |--------|------------|-------------------------|
    /// | 0      | `u8`       | Magic: 0x1C             |
    /// | 1      | `Filename` | A [Filename] structure. |
    #[brw(magic(28u8))]
    Filename(Filename),

    /// Untested
    #[brw(magic(30u8))]
    SetToFile(u16, u32),

    /// Untested
    #[brw(magic(32u8))]
    SetToLine(u32),

    /// Untested
    #[brw(magic(34u8))]
    IncrementLineNumber,

    /// Untested
    #[brw(magic(36u8))]
    IncrementLineNumberByte(u8),

    /// Untested
    #[brw(magic(38u8))]
    IncrementLineNumberWord(u32),

    /// Untested
    #[brw(magic(40u8))]
    VeryLocalSymbol(LocalSymbol),

    /// Untested
    #[brw(magic(42u8))]
    Set3ByteRegister(u16),

    /// Set MX info.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type        | Description              |
    /// |--------|-------------|--------------------------|
    /// | 0      | `u8`        | Magic: 0x2C              |
    /// | 1      | `SetMXInfo` | A [SetMXInfo] structure. |
    #[brw(magic(44u8))]
    SetMXInfo(SetMXInfo),

    /// CPU type specification.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type | Description       |
    /// |--------|------|-------------------|
    /// | 0      | `u8` | Magic: 0x2E       |
    /// | 1      | `u8` | A CPU identifier. |
    ///
    /// Constants for CPU identifiers can be found in the [cputype] module.
    #[brw(magic(46u8))]
    CPU(u8),

    /// External BSS symbol.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type | Description            |
    /// |--------|------|------------------------|
    /// | 0      | `u8` | Magic: 0x30            |
    /// | 1      | `XBSS` | An [XBSS] structure. |
    #[brw(magic(48u8))]
    XBSS(XBSS),

    //
    // Source line debugger information
    //
    /// Increment line number.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type  | Description                                  |
    /// |--------|-------|----------------------------------------------|
    /// | 0      | `u8`  | Magic: 0x32                                  |
    /// | 1      | `u16` | The offset where the new line number starts. |
    ///
    /// See also: [IncSLDLineNumByte](Self::IncSLDLineNumByte), [IncSLDLineNumWord](Self::IncSLDLineNumWord)
    #[brw(magic(50u8))]
    IncSLDLineNum(u16),

    /// Increment line number by an amount.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type  | Description                                  |
    /// |--------|-------|----------------------------------------------|
    /// | 0      | `u8`  | Magic: 0x34                                  |
    /// | 1      | `u16` | The offset where the new line number starts. |
    /// | 3      | `u8`  | The amount to increment the line number.     |
    ///
    /// See also [IncSLDLineNum](Self::IncSLDLineNum), [IncSLDLineNumWord](Self::IncSLDLineNumWord)
    #[brw(magic(52u8))]
    IncSLDLineNumByte(u16, u8),

    /// Increment line number by an amount.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type  | Description                                  |
    /// |--------|-------|----------------------------------------------|
    /// | 0      | `u8`  | Magic: 0x34                                  |
    /// | 1      | `u16` | The offset where the new line number starts. |
    /// | 3      | `u8`  | The amount to increment the line number.     |
    ///
    /// See also: [IncSLDLineNum](Self::IncSLDLineNum), [IncSLDLineNumByte](Self::IncSLDLineNumByte)
    #[brw(magic(54u8))]
    IncSLDLineNumWord(u16, u32),

    /// Set line number.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type            | Description                  |
    /// |--------|-----------------|------------------------------|
    /// | 0      | `u8`            | Magic: 0x38                  |
    /// | 1      | `SetSLDLineNum` | A [SetSLDLineNum] structure. |
    ///
    /// See also: [SetSLDLineNumFile](Self::SetSLDLineNumFile)
    #[brw(magic(56u8))]
    SetSLDLineNum(SetSLDLineNum),

    /// Set line number with file name.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type                | Description                      |
    /// |--------|---------------------|----------------------------------|
    /// | 0      | `u8`                | Magic: 0x3A                      |
    /// | 1      | `SetSLDLineNumFile` | A [SetSLDLineNumFile] structure. |
    ///
    /// See also: [SetSLDLineNum](Self::SetSLDLineNum)
    #[brw(magic(58u8))]
    SetSLDLineNumFile(SetSLDLineNumFile),

    /// End of SLD info.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type            | Description                 |
    /// |--------|-----------------|-----------------------------|
    /// | 0      | `u8`            | Magic: 0x3C                 |
    /// | 1      | `u16`           | Offset where SLD info ends. |
    #[brw(magic(60u8))]
    EndSLDInfo(u16),

    /// Untested
    #[brw(magic(62u8))]
    RepeatByte(u32),

    /// Untested
    #[brw(magic(64u8))]
    RepeatWord(u32),

    /// Untested
    #[brw(magic(66u8), assert(unimplemented("ProcedureCall")))]
    RepeatLong(u32),

    /// Untested
    #[brw(magic(68u8), assert(unimplemented("ProcedureCall")))]
    ProcedureCall(ProcedureCall),

    /// Untested
    #[brw(magic(70u8), assert(unimplemented("ProcedureDefinition")))]
    ProcedureDefinition(ProcedureDefinition),

    /// Untested
    #[brw(magic(72u8))]
    Repeat3Byte(u32),

    //
    // Function and block debug information
    //
    /// Function start marker.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type            | Description                  |
    /// |--------|-----------------|------------------------------|
    /// | 0      | `u8`            | Magic: 0x4A                  |
    /// | 1      | `FunctionStart` | A [FunctionStart] structure. |
    #[brw(magic(74u8))]
    FunctionStart(FunctionStart),

    /// Function end marker.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type                | Description                      |
    /// |--------|---------------------|----------------------------------|
    /// | 0      | `u8`                | Magic: 0x4C                      |
    /// | 1      | `SectionOffsetLine` | A [SectionOffsetLine] structure. |
    #[brw(magic(76u8))]
    FunctionEnd(SectionOffsetLine),

    /// Block start marker.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type                | Description                      |
    /// |--------|---------------------|----------------------------------|
    /// | 0      | `u8`                | Magic: 0x4E                      |
    /// | 1      | `SectionOffsetLine` | A [SectionOffsetLine] structure. |
    #[brw(magic(78u8))]
    BlockStart(SectionOffsetLine),

    /// Block end marker.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type                | Description                      |
    /// |--------|---------------------|----------------------------------|
    /// | 0      | `u8`                | Magic: 0x50                      |
    /// | 1      | `SectionOffsetLine` | A [SectionOffsetLine] structure. |
    #[brw(magic(80u8))]
    BlockEnd(SectionOffsetLine),

    //
    // Type and variable definitions
    //
    /// Variable/type definition.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type  | Description        |
    /// |--------|-------|--------------------|
    /// | 0      | `u8`  | Magic: 0x52        |
    /// | 1      | `Def` | A [Def] structure. |
    #[brw(magic(82u8))]
    Def(Def),

    /// Extended definition with tag.
    ///
    /// # Structure on Disk
    ///
    /// | Offset | Type   | Description         |
    /// |--------|--------|---------------------|
    /// | 0      | `u8`   | Magic: 0x54         |
    /// | 1      | `Def2` | A [Def2] structure. |
    #[brw(magic(84u8))]
    Def2(Def2),
}

/// Returns true if the LC_ALL or LANG environment variable indicates British English.
fn is_en_gb() -> bool {
    let lang = if let Ok(l) = std::env::var("LC_ALL") {
        l
    } else if let Ok(l) = std::env::var("LANG") {
        l
    } else {
        "".to_string()
    };

    lang.starts_with("en_GB")
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_with_options(f, &display::Options::default())
    }
}

impl display::DisplayWithOptions for Section {
    fn fmt_with_options(&self, f: &mut fmt::Formatter, options: &display::Options) -> fmt::Result {
        options.write_indent(f)?;
        match self {
            Self::NOP => write!(f, "0 : End of file"),
            Self::Code(code) => {
                write!(f, "2 : Code {} bytes", code.code.len())?;
                match options.code_format {
                    display::CodeFormat::Disassembly => {
                        writeln!(f, "\n")?;
                        for instruction in code.code.chunks(4) {
                            if instruction.len() == 4 {
                                let ins = u32::from_le_bytes(instruction.try_into().unwrap());
                                let asm = Instruction::new(ins, 0x80000000, InstrCategory::CPU)
                                    .disassemble(None, 0);
                                options.write_indent(f)?;
                                writeln!(f, "    /* {ins:08x} */   {asm}")?;
                            } else {
                                write!(f, "    /* ")?;
                                for byte in instruction {
                                    write!(f, "{byte:02x}")?;
                                }
                                writeln!(f, " */ ; invalid")?;
                            }
                        }
                    }
                    display::CodeFormat::Hex => {
                        writeln!(f, "\n")?;
                        for (i, chunk) in code.code.chunks(16).enumerate() {
                            options.write_indent(f)?;
                            write!(f, "{:04x}:", i * 16)?;
                            for byte in chunk {
                                write!(f, " {:02x}", byte)?;
                            }
                            writeln!(f)?;
                        }
                    }
                    display::CodeFormat::None => (),
                }
                Ok(())
            }
            Self::RunAtOffset(section_id, offset) => {
                write!(f, "4 : Run at offset {offset:x} in {section_id:x}")
            }
            Self::SectionSwitch(section_id) => write!(f, "6 : Switch to section {section_id:x}"),
            Self::BSS(size) => {
                let uninit = if is_en_gb() {
                    "Uninitialised"
                } else {
                    "Uninitialized"
                };
                write!(f, "8 : {} data, {} bytes", uninit, size)
            }
            Self::Patch(patch) => write!(
                f,
                "10 : Patch type {} at offset {:x} with {}",
                patch.tag, patch.offset, patch.expression
            ),
            Self::XDEF(xdef) => write!(
                f,
                "12 : XDEF symbol number {:x} '{}' at offset {:x} in section {:x}",
                xdef.number,
                xdef.symbol_name(),
                xdef.offset,
                xdef.section
            ),
            Self::XREF(xref) => write!(
                f,
                "14 : XREF symbol number {:x} '{}'",
                xref.number,
                xref.symbol_name()
            ),
            Self::LNKHeader(section) => write!(
                f,
                "16 : Section symbol number {:x} '{}' in group {} alignment {}",
                section.section,
                section.type_name(),
                section.group,
                section.align
            ),
            Self::LocalSymbol(symbol) => write!(
                f,
                "18 : Local symbol '{}' at offset {:x} in section {:x}",
                symbol.name(),
                symbol.offset,
                symbol.section
            ),
            Self::GroupSymbol(symbol) => write!(
                f,
                "20 : Group symbol number {:x} `{}` type {}",
                symbol.number,
                symbol.name(),
                symbol.sym_type,
            ),
            Self::ByteSizeRegister(register) => {
                write!(f, "22 : Set byte size register to reg offset {register}",)
            }
            Self::WordSizeRegister(register) => {
                write!(f, "24 : Set word size register to reg offset {register}",)
            }
            Self::LongSizeRegister(register) => {
                write!(f, "26 : Set long size register to reg offset {register}",)
            }
            Self::Filename(filename) => write!(
                f,
                "28 : Define file number {:x} as \"{}\"",
                filename.number,
                filename.name()
            ),
            Self::SetToFile(file, line) => write!(f, "30 : Set to {file:x}, line {line}",),
            Self::SetToLine(line) => write!(f, "32 : Set to line {line}",),
            Self::IncrementLineNumber => write!(f, "34 : Increment line number",),
            Self::IncrementLineNumberByte(num) => write!(f, "36 : Increment line number by {num}",),
            Self::IncrementLineNumberWord(num) => write!(f, "38 : Increment line number by {num}",),
            Self::VeryLocalSymbol(symbol) => write!(
                f,
                "40 : Very local symbol '{}' at offset {:x} in section {:x}",
                symbol.name(),
                symbol.offset,
                symbol.section,
            ),
            Self::Set3ByteRegister(register) => {
                write!(f, "42 : Set 3-byte size register to reg offset {register}",)
            }
            Self::SetMXInfo(set_mx_info) => write!(
                f,
                "44 : Set MX info at offset {:x} to {:x}",
                set_mx_info.offset, set_mx_info.value,
            ),
            Self::CPU(cpu) => write!(f, "46 : Processor type {}", { *cpu }),
            Self::XBSS(xbss) => write!(
                f,
                "48 : XBSS symbol number {:x} '{}' size {:x} in section {:x}",
                xbss.number,
                xbss.name(),
                xbss.size,
                xbss.section
            ),
            Self::IncSLDLineNum(offset) => write!(f, "50 : Inc SLD linenum at offset {offset:x}"),
            Self::IncSLDLineNumByte(offset, byte) => write!(
                f,
                "52 : Inc SLD linenum by byte {byte} at offset {offset:x}"
            ),
            Self::IncSLDLineNumWord(offset, word) => write!(
                f,
                "54 : Inc SLD linenum by word {word} at offset {offset:x}"
            ),
            Self::SetSLDLineNum(line) => write!(
                f,
                "56 : Set SLD linenum to {} at offset {:x}",
                line.linenum, line.offset
            ),
            Self::SetSLDLineNumFile(line) => write!(
                f,
                "58 : Set SLD linenum to {} at offset {:x} in file {:x}",
                line.linenum, line.offset, line.file
            ),
            Self::EndSLDInfo(offset) => write!(f, "60 : End SLD info at offset {offset:x}"),

            Self::RepeatByte(count) => write!(f, "62 : Repeat byte {count} times"),
            Self::RepeatWord(count) => write!(f, "64 : Repeat word {count} times"),
            Self::RepeatLong(count) => write!(f, "66 : Repeat long {count} times"),
            Self::ProcedureCall(call) => write!(f, "68 : <<<<Unimplemented>>>> {:?}", call),
            Self::ProcedureDefinition(definition) => {
                write!(f, "68 : <<<<Unimplemented>>>> {:?}", definition)
            }
            Self::Repeat3Byte(count) => write!(f, "70 : Repeat 3-byte {count} times"),
            Self::FunctionStart(start) => write!(
                f,
                "74 : Function start :\n\
                \x20 section {:04x}\n\
                \x20 offset ${:08x}\n\
                \x20 file {:04x}\n\
                \x20 start line {}\n\
                \x20 frame reg {}\n\
                \x20 frame size {}\n\
                \x20 return pc reg {}\n\
                \x20 mask ${:08x}\n\
                \x20 mask offset {}\n\
                \x20 name {}",
                start.section,
                start.offset,
                start.file,
                start.linenum,
                start.frame_register,
                start.frame_size,
                start.return_pc_register,
                start.mask,
                start.mask_offset,
                start.name()
            ),
            Self::FunctionEnd(end) => write!(
                f,
                "76 : Function end :\n\
                \x20 section {:04x}\n\
                \x20 offset ${:08x}\n\
                \x20 end line {}",
                end.section, end.offset, end.linenum
            ),
            // n.b.! the missing newline before section is intentional to match the output of OBJDUMP.EXE
            Self::BlockStart(start) => write!(
                f,
                "78 : Block start :\
                \x20 section {:04x}\n\
                \x20 offset ${:08x}\n\
                \x20 start line {}",
                start.section, start.offset, start.linenum
            ),
            Self::BlockEnd(end) => write!(
                f,
                "80 : Block end\n\
                \x20 section {:04x}\n\
                \x20 offset ${:08x}\n\
                \x20 end line {}",
                end.section, end.offset, end.linenum
            ),
            Self::Def(def) => write!(
                f,
                "82 : Def :\n\
                \x20 section {:04x}\n\
                \x20 value ${:08x}\n\
                \x20 class {}\n\
                \x20 type {}\n\
                \x20 size {}\n\
                \x20 name : {}",
                def.section,
                def.value,
                def.class,
                def.def_type,
                def.size,
                def.name()
            ),
            Self::Def2(def) => write!(
                f,
                "84 : Def2 :\n\
                \x20 section {:04x}\n\
                \x20 value ${:08x}\n\
                \x20 class {}\n\
                \x20 type {}\n\
                \x20 size {}\n\
                \x20 dims {} \n\
                \x20 tag {}\n\
                {}",
                def.section,
                def.value,
                def.class,
                def.def_type,
                def.size,
                def.dims,
                def.tag(),
                def.name()
            ),
        }
    }
}

#[cfg(test)]
mod test {
    use std::ffi::OsStr;
    use std::time::UNIX_EPOCH;

    use super::*;
    use binrw::io::Cursor;
    use binrw::{BinRead, BinWrite};

    #[test]
    fn test_datetime() {
        let t: u32 = 0x813320af;
        let dt = NaiveDateTime::from_psyq_timestamp(t).expect("datetime");
        assert_eq!(dt.year_ce().1, 1996);
        assert_eq!(dt.month(), 5);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 16);
        assert_eq!(dt.minute(), 9);
        assert_eq!(dt.second(), 38);
        assert_eq!(t, dt.to_psyq_timestamp());
        let st = SystemTime::from_psyq_timestamp(t).expect("systemtime");
        assert_eq!(
            832176578u64,
            st.duration_since(UNIX_EPOCH).expect("duration").as_secs()
        );
        assert_eq!(t, st.to_psyq_timestamp());

        let t: u32 = 0x8d061f4c;
        let dt = NaiveDateTime::from_psyq_timestamp(t).expect("datetime");
        assert_eq!(dt.year_ce().1, 1995);
        assert_eq!(dt.month(), 10);
        assert_eq!(dt.day(), 12);
        assert_eq!(dt.hour(), 17);
        assert_eq!(dt.minute(), 40);
        assert_eq!(dt.second(), 12);
        assert_eq!(t, dt.to_psyq_timestamp());
        let st = SystemTime::from_psyq_timestamp(t).expect("systemtime");
        assert_eq!(
            813519612u64,
            st.duration_since(UNIX_EPOCH).expect("duration").as_secs()
        );
        assert_eq!(t, st.to_psyq_timestamp());
    }

    #[test]
    fn test_path_to_module_name() {
        assert_eq!(
            *b"OUTPUT  ",
            path_to_module_name(Path::new("some/output.obj"))
        );
        assert_eq!(
            *b"LONGNAME",
            path_to_module_name(Path::new("some/longname.obj"))
        );
        // name is truncated to 8 characters
        assert_eq!(
            *b"LONGERNA",
            path_to_module_name(Path::new("some/longername.obj"))
        );
        // strings with code points that fit into 8-bytes are "fine"
        let name: [u8; 8] = "👾    ".as_bytes().try_into().unwrap();
        assert_eq!(name, path_to_module_name(Path::new("some/👾.obj")));
        // strings with code points that are split are not
        let name: [u8; 8] = "👾☕ ".as_bytes().try_into().unwrap();
        assert_eq!(name, path_to_module_name(Path::new("some/👾☕☕.obj")));
        // all 8-bytes consumed by multi-byte
        let name: [u8; 8] = "👾👾".as_bytes().try_into().unwrap();
        assert_eq!(name, path_to_module_name(Path::new("some/👾👾.obj")));
        // diacritics
        let name: [u8; 8] = "A͢B    ".as_bytes().try_into().unwrap();
        assert_eq!(name, path_to_module_name(Path::new("some/a͢b.obj")));
    }

    #[test]
    #[should_panic]
    fn test_path_to_module_name_missing_file_name() {
        path_to_module_name(Path::new("."));
    }

    #[test]
    #[should_panic]
    fn test_path_to_module_name_invalid_unicode() {
        // b"\u{C0}invalid.obj"
        let s: &OsStr;
        unsafe {
            s = OsStr::from_encoded_bytes_unchecked(&[
                0xC0, 0x69, 0x6E, 0x76, 0x61, 0x6C, 0x69, 0x64, 0x2e, 0x6f, 0x62, 0x6a,
            ]);
        }
        path_to_module_name(Path::new(s));
    }

    #[test]
    fn test_lib() {
        let bytes = b"\
           \x4C\x49\x42\x01\x41\x35\x36\x20\x20\x20\x20\x20\xAF\x20\x2C\x81\
           \x1A\x00\x00\x00\x8E\x00\x00\x00\x04\x65\x78\x69\x74\x00\x4C\x4E\
           \x4B\x02\x2E\x07\x10\x04\xF0\x00\x00\x08\x06\x2E\x72\x64\x61\x74\
           \x61\x10\x00\xF0\x00\x00\x08\x05\x2E\x74\x65\x78\x74\x10\x01\xF0\
           \x00\x00\x08\x05\x2E\x64\x61\x74\x61\x10\x03\xF0\x00\x00\x08\x06\
           \x2E\x73\x64\x61\x74\x61\x10\x05\xF0\x00\x00\x08\x04\x2E\x62\x73\
           \x73\x10\x02\xF0\x00\x00\x08\x05\x2E\x73\x62\x73\x73\x0C\x01\x00\
           \x00\xF0\x00\x00\x00\x00\x04\x65\x78\x69\x74\x06\x00\xF0\x02\x10\
           \x00\xB0\x00\x0A\x24\x08\x00\x40\x01\x38\x00\x09\x24\x00\x00\x00\
           \x00\x00"
            .to_vec();
        //.0.  1.  2.  3.  4.  5.  6.  7.  8.  9.  A.  B.  C.  D.  E.  F.
        let mut data = Cursor::new(&bytes);
        let lib = LIB::read(&mut data).unwrap();
        assert_eq!(lib.version, 1);
        // assert_eq!(lib.modules().len(), 1);

        let module = lib.modules().first().expect("modules[0]");
        assert_eq!(module.obj.version(), 2);
        assert_eq!(module.name(), "A56");
        assert_eq!(module.metadata.created, 2167152815);
        assert_eq!(module.metadata.offset, 26);
        assert_eq!(module.metadata.size, 142);
        assert_eq!(module.metadata.exports.len(), 2);
        assert_eq!(module.exports().len(), 1);

        let export = module
            .metadata
            .exports
            .first()
            .expect("modules[0].exports[0]");
        assert_eq!(export.name_size, 4);
        assert_eq!(export.name(), "exit");

        let lnk = &module.obj;
        assert_eq!(lnk.version, 2);

        let Section::CPU(cpu) = lnk.sections.first().expect("module[0].obj.sections[0]") else {
            unreachable!();
        };
        assert_eq!(*cpu, cputype::MIPS_R3000);
        /*
                assert_eq!(section.section, 61444);
                assert_eq!(section.group, 0);
                assert_eq!(section.align, 8);
                assert_eq!(section.type_name_size, 6);
                assert_eq!(section.type_name(), ".rdata");
        */

        assert_eq!(data.position(), bytes.len() as u64);

        // roundtrip
        let mut writer = Cursor::new(Vec::new());
        lib.write_le(&mut writer).unwrap();
        assert_eq!(writer.into_inner(), bytes);
    }

    #[test]
    fn test_object_entry() {
        let bytes = b"\
            \x53\x50\x52\x49\x4E\x54\x46\x20\xAF\x20\x33\x81\x1D\x00\x00\x00\
            \x25\x0E\x00\x00\x07\x73\x70\x72\x69\x6E\x74\x66\x00\x4C\x4E\x4B\
            \x02\x2E\x07\x10\x01\x00\x00\x00\x08\x06\x2E\x72\x64\x61\x74\x61\
            \x10\x02\x00\x00\x00\x08\x05\x2E\x74\x65\x78\x74\x10\x03\x00\x00\
            \x00\x08\x05\x2E\x64\x61\x74\x61\x10\x04\x00\x00\x00\x08\x06\x2E\
            \x73\x64\x61\x74\x61\x10\x05\x00\x00\x00\x08\x05\x2E\x73\x62\x73\
            \x73\x10\x06\x00\x00\x00\x08\x04\x2E\x62\x73\x73\x10\x07\x00\x00\
            \x00\x08\x06\x2E\x63\x74\x6F\x72\x73\x10\x08\x00\x00\x00\x08\x06\
            \x2E\x64\x74\x6F\x72\x73\x1C\x09\x00\x17\x43\x3A\x5C\x50\x53\x58\
            \x5C\x53\x52\x43\x5C\x43\x32\x5C\x53\x50\x52\x49\x4E\x54\x46\x2E\
            \x43\x06\x02\x00\x06\x03\x00\x02\x01\x00\x00\x08\x0B\x00\x00\x00\
            \x06\x01\x00\x02\x25\x00\x30\x31\x32\x33\x34\x35\x36\x37\x38\x39\
            \x41\x42\x43\x44\x45\x46\x00\x00\x00\x00\x30\x31\x32\x33\x34\x35\
            \x36\x37\x38\x39\x61\x62\x63\x64\x65\x66\x00\x06\x02\x00\x02\xC8\
            \x02\x04\x00\xA5\xAF\x08\x00\xA6\xAF\x0C\x00\xA7\xAF\xB8\xFD\xBD\
            \x27\x34\x02\xB3\xAF\x21\x98\x80\x00\x50\x02\xA2\x27\x44\x02\xBF\
            \xAF\x40\x02\xB6\xAF\x3C\x02\xB5\xAF\x38\x02\xB4\xAF\x30\x02\xB2\
            \xAF\x2C\x02\xB1\xAF\x28\x02\xB0\xAF\x4C\x02\xA5\xAF\x20\x02\xA2\
            \xAF\x00\x00\xA5\x90\x00\x00\x00\x00\xF6\x01\xA0\x10\x21\x90\x00\
            \x00\x2D\x00\x16\x34\x2B\x00\x15\x34\x20\x00\x14\x34\x25\x00\x02\
            \x34\xC0\x01\xA2\x14\x21\x10\x72\x02\x00\x00\x05\x3C\x00\x00\xA5\
            \x24\x00\x00\xA2\x8C\x04\x00\xA3\x8C\x08\x00\xA4\x8C\x10\x02\xA2\
            \xAF\x14\x02\xA3\xAF\x18\x02\xA4\xAF\x23\x00\x06\x34\x30\x00\x03\
            \x34\x4C\x02\xA4\x8F\x00\x00\x00\x00\x01\x00\x82\x24\x4C\x02\xA2\
            \xAF\x01\x00\x85\x90\x00\x00\x00\x00\x06\x00\xB6\x14\x00\x00\x00\
            \x00\x10\x02\xA2\x8F\x00\x00\x00\x00\x01\x00\x42\x34\x00\x00\x00\
            \x08\x10\x02\xA2\xAF\x06\x00\xB5\x14\x00\x00\x00\x00\x10\x02\xA2\
            \x8F\x00\x00\x00\x00\x02\x00\x42\x34\x00\x00\x00\x08\x10\x02\xA2\
            \xAF\x03\x00\xB4\x14\x00\x00\x00\x00\x00\x00\x00\x08\x11\x02\xA5\
            \xA3\x06\x00\xA6\x14\x00\x00\x00\x00\x10\x02\xA2\x8F\x00\x00\x00\
            \x00\x04\x00\x42\x34\x00\x00\x00\x08\x10\x02\xA2\xAF\x06\x00\xA3\
            \x14\x2A\x00\x02\x34\x10\x02\xA2\x8F\x00\x00\x00\x00\x08\x00\x42\
            \x34\x00\x00\x00\x08\x10\x02\xA2\xAF\x22\x00\xA2\x14\xD0\xFF\xA2\
            \x24\x20\x02\xA3\x8F\x00\x00\x00\x00\x04\x00\x62\x24\x20\x02\xA2\
            \xAF\x00\x00\x62\x8C\x00\x00\x00\x00\x06\x00\x41\x04\x14\x02\xA2\
            \xAF\x10\x02\xA3\x8F\x23\x10\x02\x00\x14\x02\xA2\xAF\x01\x00\x63\
            \x34\x10\x02\xA3\xAF\x02\x00\x82\x24\x4C\x02\xA2\xAF\x02\x00\x85\
            \x90\x00\x00\x00\x08\x2E\x00\x02\x34\x14\x02\xA3\x8F\x00\x00\x00\
            \x00\x80\x10\x03\x00\x21\x10\x43\x00\x40\x10\x02\x00\xD0\xFF\x42\
            \x24\x21\x10\x45\x00\x14\x02\xA2\xAF\x4C\x02\xA3\x8F\x00\x00\x00\
            \x00\x01\x00\x62\x24\x4C\x02\xA2\xAF\x01\x00\x65\x90\x00\x00\x00\
            \x00\xD0\xFF\xA2\x24\x0A\x00\x42\x2C\xEF\xFF\x40\x14\x2E\x00\x02\
            \x34\x2F\x00\xA2\x14\x00\x00\x00\x00\x4C\x02\xA4\x8F\x00\x00\x00\
            \x00\x01\x00\x82\x24\x4C\x02\xA2\xAF\x01\x00\x85\x90\x2A\x00\x02\
            \x34\x1C\x00\xA2\x14\xD0\xFF\xA2\x24\x20\x02\xA3\x8F\x00\x00\x00\
            \x00\x04\x00\x62\x24\x20\x02\xA2\xAF\x00\x00\x62\x8C\x00\x00\x00\
            \x00\x18\x02\xA2\xAF\x02\x00\x82\x24\x4C\x02\xA2\xAF\x02\x00\x85\
            \x90\x00\x00\x00\x08\x00\x00\x00\x00\x18\x02\xA3\x8F\x00\x00\x00\
            \x00\x80\x10\x03\x00\x21\x10\x43\x00\x40\x10\x02\x00\xD0\xFF\x42\
            \x24\x21\x10\x45\x00\x18\x02\xA2\xAF\x4C\x02\xA3\x8F\x00\x00\x00\
            \x00\x01\x00\x62\x24\x4C\x02\xA2\xAF\x01\x00\x65\x90\x00\x00\x00\
            \x00\xD0\xFF\xA2\x24\x0A\x00\x42\x2C\xEF\xFF\x40\x14\x00\x00\x00\
            \x00\x18\x02\xA2\x8F\x00\x00\x00\x00\x05\x00\x40\x04\x00\x00\x00\
            \x00\x10\x02\xA2\x8F\x00\x00\x00\x00\x10\x00\x42\x34\x10\x02\xA2\
            \xAF\x10\x02\xA3\x8F\x00\x00\x00\x00\x01\x00\x62\x30\x04\x00\x40\
            \x10\x10\x02\xB1\x27\xF7\xFF\x02\x24\x24\x10\x62\x00\x10\x02\xA2\
            \xAF\xB4\xFF\xA3\x24\x2D\x00\x62\x2C\x2B\x01\x40\x10\x80\x10\x03\
            \x00\x00\x00\x01\x3C\x21\x08\x22\x00\x00\x00\x22\x8C\x00\x00\x00\
            \x00\x08\x00\x40\x00\x00\x00\x00\x00\x0A\x52\x68\x00\x2C\x04\x03\
            \x00\x00\x00\x00\x00\x00\x0A\x54\x6C\x00\x2C\x04\x03\x00\x00\x00\
            \x00\x00\x00\x0A\x4A\xBC\x00\x2C\x04\x02\x00\x00\x90\x00\x00\x00\
            \x0A\x4A\xD8\x00\x2C\x04\x02\x00\x00\x90\x00\x00\x00\x0A\x4A\xE8\
            \x00\x2C\x04\x02\x00\x00\x90\x00\x00\x00\x0A\x4A\x04\x01\x2C\x04\
            \x02\x00\x00\x90\x00\x00\x00\x0A\x4A\x20\x01\x2C\x04\x02\x00\x00\
            \x90\x00\x00\x00\x0A\x4A\x70\x01\x2C\x04\x02\x00\x00\xC0\x01\x00\
            \x00\x0A\x4A\x10\x02\x2C\x04\x02\x00\x00\x60\x02\x00\x00\x0A\x52\
            \xB0\x02\x2C\x04\x01\x00\x00\x28\x00\x00\x00\x0A\x54\xB8\x02\x2C\
            \x04\x01\x00\x00\x28\x00\x00\x00\x06\x01\x00\x02\xB7\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x0A\x10\x03\x00\x2C\x04\x02\x00\x00\xE0\x02\
            \x00\x00\x0A\x10\x07\x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\x0A\
            \x10\x0B\x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\x0A\x10\x0F\x00\
            \x2C\x04\x02\x00\x00\x58\x07\x00\x00\x0A\x10\x13\x00\x2C\x04\x02\
            \x00\x00\x58\x07\x00\x00\x0A\x10\x17\x00\x2C\x04\x02\x00\x00\x58\
            \x07\x00\x00\x0A\x10\x1B\x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\
            \x0A\x10\x1F\x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\x0A\x10\x23\
            \x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\x0A\x10\x27\x00\x2C\x04\
            \x02\x00\x00\x58\x07\x00\x00\x0A\x10\x2B\x00\x2C\x04\x02\x00\x00\
            \x58\x07\x00\x00\x0A\x10\x2F\x00\x2C\x04\x02\x00\x00\x58\x07\x00\
            \x00\x0A\x10\x33\x00\x2C\x04\x02\x00\x00\x78\x05\x00\x00\x0A\x10\
            \x37\x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\x0A\x10\x3B\x00\x2C\
            \x04\x02\x00\x00\x58\x07\x00\x00\x0A\x10\x3F\x00\x2C\x04\x02\x00\
            \x00\x58\x07\x00\x00\x0A\x10\x43\x00\x2C\x04\x02\x00\x00\x58\x07\
            \x00\x00\x0A\x10\x47\x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\x0A\
            \x10\x4B\x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\x0A\x10\x4F\x00\
            \x2C\x04\x02\x00\x00\x58\x07\x00\x00\x0A\x10\x53\x00\x2C\x04\x02\
            \x00\x00\x58\x07\x00\x00\x0A\x10\x57\x00\x2C\x04\x02\x00\x00\x58\
            \x07\x00\x00\x0A\x10\x5B\x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\
            \x0A\x10\x5F\x00\x2C\x04\x02\x00\x00\x80\x06\x00\x00\x0A\x10\x63\
            \x00\x2C\x04\x02\x00\x00\x0C\x03\x00\x00\x0A\x10\x67\x00\x2C\x04\
            \x02\x00\x00\x58\x07\x00\x00\x0A\x10\x6B\x00\x2C\x04\x02\x00\x00\
            \x58\x07\x00\x00\x0A\x10\x6F\x00\x2C\x04\x02\x00\x00\x58\x07\x00\
            \x00\x0A\x10\x73\x00\x2C\x04\x02\x00\x00\xC8\x02\x00\x00\x0A\x10\
            \x77\x00\x2C\x04\x02\x00\x00\x0C\x03\x00\x00\x0A\x10\x7B\x00\x2C\
            \x04\x02\x00\x00\x58\x07\x00\x00\x0A\x10\x7F\x00\x2C\x04\x02\x00\
            \x00\x58\x07\x00\x00\x0A\x10\x83\x00\x2C\x04\x02\x00\x00\xD4\x02\
            \x00\x00\x0A\x10\x87\x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\x0A\
            \x10\x8B\x00\x2C\x04\x02\x00\x00\x24\x07\x00\x00\x0A\x10\x8F\x00\
            \x2C\x04\x02\x00\x00\x74\x04\x00\x00\x0A\x10\x93\x00\x2C\x04\x02\
            \x00\x00\x64\x05\x00\x00\x0A\x10\x97\x00\x2C\x04\x02\x00\x00\x58\
            \x07\x00\x00\x0A\x10\x9B\x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\
            \x0A\x10\x9F\x00\x2C\x04\x02\x00\x00\xA0\x06\x00\x00\x0A\x10\xA3\
            \x00\x2C\x04\x02\x00\x00\x58\x07\x00\x00\x0A\x10\xA7\x00\x2C\x04\
            \x02\x00\x00\x5C\x03\x00\x00\x0A\x10\xAB\x00\x2C\x04\x02\x00\x00\
            \x58\x07\x00\x00\x0A\x10\xAF\x00\x2C\x04\x02\x00\x00\x58\x07\x00\
            \x00\x0A\x10\xB3\x00\x2C\x04\x02\x00\x00\x88\x05\x00\x00\x06\x02\
            \x00\x02\x94\x05\x10\x02\xA2\x8F\x00\x00\x00\x08\x20\x00\x42\x34\
            \x10\x02\xA2\x8F\x00\x00\x00\x08\x40\x00\x42\x34\x10\x02\xA2\x8F\
            \x00\x00\x00\x00\x80\x00\x42\x34\x10\x02\xA2\xAF\x4C\x02\xA3\x8F\
            \x00\x00\x00\x00\x01\x00\x62\x24\x4C\x02\xA2\xAF\x01\x00\x65\x90\
            \x00\x00\x00\x08\xB4\xFF\xA3\x24\x20\x02\xA3\x8F\x00\x00\x00\x00\
            \x04\x00\x62\x24\x20\x02\xA2\xAF\x00\x00\x64\x8C\x10\x02\xA3\x8F\
            \x00\x00\x00\x00\x20\x00\x62\x30\x02\x00\x40\x10\x00\x14\x04\x00\
            \x03\x24\x02\x00\x04\x00\x81\x04\x02\x00\x62\x30\x23\x20\x04\x00\
            \x00\x00\x00\x08\x11\x02\xB6\xA3\x0E\x00\x40\x10\x00\x00\x00\x00\
            \x00\x00\x00\x08\x11\x02\xB5\xA3\x20\x02\xA3\x8F\x00\x00\x00\x00\
            \x04\x00\x62\x24\x20\x02\xA2\xAF\x00\x00\x64\x8C\x10\x02\xA2\x8F\
            \x00\x00\x00\x00\x20\x00\x42\x30\x02\x00\x40\x10\x11\x02\xA0\xA3\
            \xFF\xFF\x84\x30\x10\x02\xA3\x8F\x00\x00\x00\x00\x10\x00\x62\x30\
            \x0F\x00\x40\x14\x08\x00\x62\x30\x08\x00\x40\x10\x00\x00\x00\x00\
            \x14\x02\xA3\x8F\x11\x02\xA2\x93\x00\x00\x00\x00\x03\x00\x40\x10\
            \x18\x02\xA3\xAF\xFF\xFF\x62\x24\x18\x02\xA2\xAF\x18\x02\xA2\x8F\
            \x00\x00\x00\x00\x02\x00\x40\x1C\x01\x00\x02\x34\x18\x02\xA2\xAF\
            \x10\x00\x80\x10\x21\x80\x00\x00\xCC\xCC\x05\x3C\xCD\xCC\xA5\x34\
            \x19\x00\x85\x00\xFF\xFF\x31\x26\x01\x00\x10\x26\x10\x18\x00\x00\
            \xC2\x18\x03\x00\x80\x10\x03\x00\x21\x10\x43\x00\x40\x10\x02\x00\
            \x23\x10\x82\x00\x30\x00\x42\x24\x21\x20\x60\x00\xF4\xFF\x80\x14\
            \x00\x00\x22\xA2\x18\x02\xA2\x8F\x00\x00\x00\x00\x2A\x10\x02\x02\
            \x0A\x00\x40\x10\x00\x00\x00\x00\x30\x00\x03\x34\xFF\xFF\x31\x26\
            \x00\x00\x23\xA2\x18\x02\xA2\x8F\x01\x00\x10\x26\x2A\x10\x02\x02\
            \xFB\xFF\x40\x14\xFF\xFF\x31\x26\x01\x00\x31\x26\x11\x02\xA2\x93\
            \x00\x00\x00\x00\xC5\x00\x40\x10\x00\x00\x00\x00\xFF\xFF\x31\x26\
            \x11\x02\xA2\x93\x01\x00\x10\x26\x00\x00\x00\x08\x00\x00\x22\xA2\
            \x20\x02\xA3\x8F\x00\x00\x00\x00\x04\x00\x62\x24\x20\x02\xA2\xAF\
            \x00\x00\x64\x8C\x10\x02\xA3\x8F\x00\x00\x00\x00\x20\x00\x62\x30\
            \x02\x00\x40\x10\x10\x00\x62\x30\xFF\xFF\x84\x30\x0B\x00\x40\x14\
            \x08\x00\x62\x30\x04\x00\x40\x10\x00\x00\x00\x00\x14\x02\xA2\x8F\
            \x00\x00\x00\x00\x18\x02\xA2\xAF\x18\x02\xA2\x8F\x00\x00\x00\x00\
            \x02\x00\x40\x1C\x01\x00\x02\x34\x18\x02\xA2\xAF\x08\x00\x80\x10\
            \x21\x80\x00\x00\xFF\xFF\x31\x26\x07\x00\x82\x30\x30\x00\x42\x24\
            \x00\x00\x22\xA2\xC2\x20\x04\x00\xFA\xFF\x80\x14\x01\x00\x10\x26\
            \x10\x02\xA2\x8F\x00\x00\x00\x00\x04\x00\x42\x30\x0A\x00\x40\x10\
            \x00\x00\x00\x00\x08\x00\x00\x12\x30\x00\x02\x34\x00\x00\x23\x92\
            \x00\x00\x00\x00\x04\x00\x62\x10\x30\x00\x02\x34\xFF\xFF\x31\x26\
            \x00\x00\x22\xA2\x01\x00\x10\x26\x18\x02\xA2\x8F\x00\x00\x00\x00\
            \x2A\x10\x02\x02\x8D\x00\x40\x10\x30\x00\x03\x34\xFF\xFF\x31\x26\
            \x00\x00\x23\xA2\x18\x02\xA2\x8F\x01\x00\x10\x26\x2A\x10\x02\x02\
            \xFB\xFF\x40\x14\xFF\xFF\x31\x26\x00\x00\x00\x08\x01\x00\x31\x26\
            \x10\x02\xA3\x8F\x08\x00\x02\x34\x18\x02\xA2\xAF\x50\x00\x63\x34\
            \x10\x02\xA3\xAF\x00\x00\x07\x3C\x00\x00\xE7\x24\x00\x00\x00\x08\
            \x00\x00\x00\x00\x00\x00\x07\x3C\x00\x00\xE7\x24\x20\x02\xA3\x8F\
            \x00\x00\x00\x00\x04\x00\x62\x24\x20\x02\xA2\xAF\x00\x00\x64\x8C\
            \x10\x02\xA3\x8F\x00\x00\x00\x00\x20\x00\x62\x30\x02\x00\x40\x10\
            \x10\x00\x62\x30\xFF\xFF\x84\x30\x0D\x00\x40\x14\x08\x00\x62\x30\
            \x06\x00\x40\x10\x04\x00\x62\x30\x14\x02\xA6\x8F\x03\x00\x40\x10\
            \x18\x02\xA6\xAF\xFE\xFF\xC2\x24\x18\x02\xA2\xAF\x18\x02\xA2\x8F\
            \x00\x00\x00\x00\x02\x00\x40\x1C\x01\x00\x02\x34\x18\x02\xA2\xAF\
            \x09\x00\x80\x10\x21\x80\x00\x00\xFF\xFF\x31\x26\x0F\x00\x82\x30\
            \x02\x21\x04\x00\x21\x10\xE2\x00\x00\x00\x42\x90\x01\x00\x10\x26\
            \xF9\xFF\x80\x14\x00\x00\x22\xA2\x18\x02\xA2\x8F\x00\x00\x00\x00\
            \x2A\x10\x02\x02\x0A\x00\x40\x10\x00\x00\x00\x00\x30\x00\x03\x34\
            \xFF\xFF\x31\x26\x00\x00\x23\xA2\x18\x02\xA2\x8F\x01\x00\x10\x26\
            \x2A\x10\x02\x02\xFB\xFF\x40\x14\xFF\xFF\x31\x26\x01\x00\x31\x26\
            \x10\x02\xA2\x8F\x00\x00\x00\x00\x04\x00\x42\x30\x43\x00\x40\x10\
            \x30\x00\x02\x34\xFF\xFF\x31\x26\x00\x00\x25\xA2\xFF\xFF\x31\x26\
            \x02\x00\x10\x26\x00\x00\x00\x08\x00\x00\x22\xA2\x20\x02\xA2\x8F\
            \xFF\xFF\x31\x26\x04\x00\x43\x24\x20\x02\xA3\xAF\x00\x00\x42\x90\
            \x01\x00\x10\x34\x00\x00\x00\x08\x00\x00\x22\xA2\x20\x02\xA2\x8F\
            \x00\x00\x00\x00\x04\x00\x43\x24\x20\x02\xA3\xAF\x10\x02\xA3\x8F\
            \x00\x00\x51\x8C\x04\x00\x62\x30\x0B\x00\x40\x10\x10\x00\x62\x30\
            \x00\x00\x30\x92\x29\x00\x40\x10\x01\x00\x31\x26\x18\x02\xA3\x8F\
            \x00\x00\x00\x00\x2A\x10\x70\x00\x24\x00\x40\x10\x00\x00\x00\x00\
            \x00\x00\x00\x08\x21\x80\x60\x00\x05\x00\x40\x14\x21\x20\x20\x02\
            \x00\x00\x00\x0C\x21\x20\x20\x02\x00\x00\x00\x08\x21\x80\x40\x00\
            \x18\x02\xA6\x8F\x00\x00\x00\x0C\x21\x28\x00\x00\x17\x00\x40\x14\
            \x23\x80\x51\x00\x18\x02\xB0\x8F\x00\x00\x00\x08\x00\x00\x00\x00\
            \x20\x02\xA2\x8F\x00\x00\x00\x00\x04\x00\x43\x24\x20\x02\xA3\xAF\
            \x10\x02\xA3\x8F\x00\x00\x51\x8C\x20\x00\x62\x30\x03\x00\x40\x10\
            \x00\x00\x00\x00\x00\x00\x00\x08\x00\x00\x32\xA6\x00\x00\x00\x08\
            \x00\x00\x32\xAE\x25\x00\x02\x34\x31\x00\xA2\x14\x21\x10\x72\x02\
            \x00\x00\x45\xA0\x00\x00\x00\x08\x01\x00\x52\x26\x14\x02\xA2\x8F\
            \x00\x00\x00\x00\x2A\x10\x02\x02\x11\x00\x40\x10\x21\x20\x72\x02\
            \x10\x02\xA2\x8F\x00\x00\x00\x00\x01\x00\x42\x30\x0D\x00\x40\x14\
            \x21\x28\x20\x02\x21\x18\x53\x02\x00\x00\x74\xA0\x01\x00\x63\x24\
            \x14\x02\xA2\x8F\x00\x00\x00\x00\xFF\xFF\x42\x24\x14\x02\xA2\xAF\
            \x2A\x10\x02\x02\xF8\xFF\x40\x14\x01\x00\x52\x26\x21\x20\x72\x02\
            \x21\x28\x20\x02\x00\x00\x00\x0C\x21\x30\x00\x02\x14\x02\xA2\x8F\
            \x00\x00\x00\x00\x2A\x10\x02\x02\x09\x00\x40\x10\x21\x90\x50\x02\
            \x21\x18\x53\x02\x00\x00\x74\xA0\x01\x00\x63\x24\x14\x02\xA2\x8F\
            \x01\x00\x10\x26\x2A\x10\x02\x02\xFA\xFF\x40\x14\x01\x00\x52\x26\
            \x4C\x02\xA3\x8F\x00\x00\x00\x00\x01\x00\x62\x24\x4C\x02\xA2\xAF\
            \x01\x00\x65\x90\x00\x00\x00\x00\x10\xFE\xA0\x14\x25\x00\x02\x34\
            \x21\x10\x72\x02\x00\x00\x40\xA0\x21\x10\x40\x02\x44\x02\xBF\x8F\
            \x40\x02\xB6\x8F\x3C\x02\xB5\x8F\x38\x02\xB4\x8F\x34\x02\xB3\x8F\
            \x30\x02\xB2\x8F\x2C\x02\xB1\x8F\x28\x02\xB0\x8F\x48\x02\xBD\x27\
            \x08\x00\xE0\x03\x00\x00\x00\x00\x0A\x4A\x04\x00\x2C\x04\x02\x00\
            \x00\xEC\x02\x00\x00\x0A\x4A\x10\x00\x2C\x04\x02\x00\x00\xEC\x02\
            \x00\x00\x0A\x4A\x3C\x00\x2C\x04\x02\x00\x00\xA4\x02\x00\x00\x0A\
            \x4A\x7C\x00\x2C\x04\x02\x00\x00\x88\x03\x00\x00\x0A\x4A\x8C\x00\
            \x2C\x04\x02\x00\x00\x88\x03\x00\x00\x0A\x4A\xA4\x01\x2C\x04\x02\
            \x00\x00\x70\x07\x00\x00\x0A\x4A\x94\x02\x2C\x04\x02\x00\x00\x70\
            \x07\x00\x00\x0A\x52\xB0\x02\x2C\x04\x01\x00\x00\x00\x00\x00\x00\
            \x0A\x54\xB4\x02\x2C\x04\x01\x00\x00\x00\x00\x00\x00\x0A\x4A\xB8\
            \x02\x2C\x04\x02\x00\x00\x90\x05\x00\x00\x0A\x52\xC0\x02\x2C\x04\
            \x01\x00\x00\x14\x00\x00\x00\x0A\x54\xC4\x02\x2C\x04\x01\x00\x00\
            \x14\x00\x00\x00\x0A\x4A\xB0\x03\x2C\x04\x02\x00\x00\x70\x07\x00\
            \x00\x0A\x4A\xD0\x03\x2C\x04\x02\x00\x00\x70\x07\x00\x00\x0A\x4A\
            \x1C\x04\x2C\x04\x02\x00\x00\x70\x07\x00\x00\x0A\x4A\x2C\x04\x02\
            \x0B\x00\x0A\x4A\x34\x04\x2C\x04\x02\x00\x00\x70\x07\x00\x00\x0A\
            \x4A\x40\x04\x02\x0C\x00\x0A\x4A\x54\x04\x2C\x04\x02\x00\x00\x70\
            \x07\x00\x00\x0A\x4A\x80\x04\x2C\x04\x02\x00\x00\x04\x08\x00\x00\
            \x0A\x4A\x88\x04\x2C\x04\x02\x00\x00\x04\x08\x00\x00\x0A\x4A\xA0\
            \x04\x2C\x04\x02\x00\x00\x04\x08\x00\x00\x0A\x4A\x00\x05\x02\x0D\
            \x00\x06\x02\x00\x0C\x0A\x00\x02\x00\x00\x00\x00\x00\x07\x73\x70\
            \x72\x69\x6E\x74\x66\x0E\x0C\x00\x06\x6D\x65\x6D\x63\x68\x72\x0E\
            \x0B\x00\x06\x73\x74\x72\x6C\x65\x6E\x0E\x0D\x00\x07\x6D\x65\x6D\
            \x6D\x6F\x76\x65\x00"
            .to_vec();
        //.0.  1.  2.  3.  4.  5.  6.  7.  8.  9.  A.  B.  C.  D.  E.  F.
        let mut data = Cursor::new(&bytes);
        let obj = Module::read(&mut data).unwrap();

        eprintln!("obj: {:?}", obj);

        assert_eq!(obj.name(), "SPRINTF");
        // assert_eq!(obj.created, 2167611567);
        // TODO: this should be based on locale
        assert_eq!(obj.created(), "15-05-96 16:09:38");
        assert_eq!(obj.metadata.offset, 29);
        assert_eq!(obj.metadata.size, 3621);
        assert_eq!(obj.metadata.exports.len(), 2);

        let export = obj.metadata.exports.first().expect("obj[0].exports[0]");
        assert_eq!(export.name_size, 7);
        assert_eq!(export.name(), "sprintf");

        let lnk = &obj.obj;
        assert_eq!(lnk.version, 2);

        let Section::CPU(cpu) = lnk.sections.first().expect("obj[0].obj.sections[0]") else {
            panic!("expected a section");
        };
        assert_eq!(*cpu, cputype::MIPS_R3000);
        /*
        assert_eq!(section.section, 1);
        assert_eq!(section.group, 0);
        assert_eq!(section.align, 8);
        assert_eq!(section.type_name_size, 6);
        assert_eq!(section.type_name(), ".rdata");
        */

        assert_eq!(data.position(), bytes.len() as u64);

        let mut writer = Cursor::new(Vec::new());
        obj.write_le(&mut writer).unwrap();
        assert_eq!(writer.into_inner(), bytes);
    }

    #[test]
    fn test_2_mbyte() {
        let bytes = b"\
            \x4C\x4E\x4B\x02\x2E\x07\x10\x08\x28\x00\x00\x08\x06\x2E\x72\x64\
            \x61\x74\x61\x10\x09\x28\x00\x00\x08\x05\x2E\x74\x65\x78\x74\x10\
            \x0A\x28\x00\x00\x08\x05\x2E\x64\x61\x74\x61\x10\x0B\x28\x00\x00\
            \x08\x06\x2E\x73\x64\x61\x74\x61\x10\x0C\x28\x00\x00\x08\x05\x2E\
            \x73\x62\x73\x73\x10\x0D\x28\x00\x00\x08\x04\x2E\x62\x73\x73\x06\
            \x08\x28\x06\x09\x28\x06\x0A\x28\x06\x0B\x28\x06\x0C\x28\x06\x0D\
            \x28\x06\x09\x28\x02\xC4\x00\x08\x00\xE0\x03\x00\x00\x00\x00\x00\
            \x00\x02\x3C\x00\x00\x42\x24\x00\x00\x03\x3C\x00\x00\x63\x24\x00\
            \x00\x40\xAC\x04\x00\x42\x24\x2B\x08\x43\x00\xFC\xFF\x20\x14\x00\
            \x00\x00\x00\x04\x00\x02\x24\x00\x00\x00\x00\x00\x00\x00\x00\x00\
            \x00\x00\x00\x00\x00\x00\x00\x00\x00\x04\x3C\x00\x00\x84\x24\x21\
            \x20\x82\x00\x00\x00\x82\x8C\x00\x80\x08\x3C\x25\xE8\x48\x00\x00\
            \x00\x04\x3C\x00\x00\x84\x24\xC0\x20\x04\x00\xC2\x20\x04\x00\x00\
            \x00\x03\x3C\x00\x00\x63\x8C\x00\x00\x00\x00\x23\x28\x43\x00\x23\
            \x28\xA4\x00\x25\x20\x88\x00\x00\x00\x01\x3C\x00\x00\x3F\xAC\x00\
            \x00\x1C\x3C\x00\x00\x9C\x27\x21\xF0\xA0\x03\x00\x00\x00\x0C\x04\
            \x00\x84\x20\x00\x00\x1F\x3C\x00\x00\xFF\x8F\x00\x00\x00\x00\x00\
            \x00\x00\x0C\x00\x00\x00\x00\x4D\x00\x00\x00\x00\x00\x20\x00\x00\
            \x00\x20\x00\x00\x00\x20\x00\x00\x00\x20\x00\x0A\x52\x08\x00\x0C\
            \x0C\x28\x0A\x54\x0C\x00\x0C\x0C\x28\x0A\x52\x10\x00\x16\x0D\x28\
            \x0A\x54\x14\x00\x16\x0D\x28\x0A\x52\x40\x00\x2C\x04\x09\x28\x00\
            \xB4\x00\x00\x00\x0A\x54\x44\x00\x2C\x04\x09\x28\x00\xB4\x00\x00\
            \x00\x0A\x52\x58\x00\x16\x0D\x28\x0A\x54\x5C\x00\x16\x0D\x28\x0A\
            \x52\x68\x00\x02\x17\x28\x0A\x54\x6C\x00\x02\x17\x28\x0A\x52\x80\
            \x00\x2C\x04\x0C\x28\x00\x00\x00\x00\x00\x0A\x54\x84\x00\x2C\x04\
            \x0C\x28\x00\x00\x00\x00\x00\x0A\x52\x88\x00\x0C\x0B\x28\x0A\x54\
            \x8C\x00\x0C\x0B\x28\x0A\x4A\x94\x00\x02\x14\x28\x0A\x52\x9C\x00\
            \x2C\x04\x0C\x28\x00\x00\x00\x00\x00\x0A\x54\xA0\x00\x2C\x04\x0C\
            \x28\x00\x00\x00\x00\x00\x0A\x4A\xA8\x00\x02\x16\x28\x06\x0C\x28\
            \x08\x04\x00\x00\x00\x0E\x14\x28\x08\x49\x6E\x69\x74\x48\x65\x61\
            \x70\x0E\x17\x28\x0A\x5F\x73\x74\x61\x63\x6B\x73\x69\x7A\x65\x0C\
            \x0F\x28\x09\x28\x08\x00\x00\x00\x10\x5F\x5F\x53\x4E\x5F\x45\x4E\
            \x54\x52\x59\x5F\x50\x4F\x49\x4E\x54\x0C\x0E\x28\x09\x28\x00\x00\
            \x00\x00\x06\x5F\x5F\x6D\x61\x69\x6E\x0E\x16\x28\x04\x6D\x61\x69\
            \x6E\x0C\x11\x28\x09\x28\xA8\x00\x00\x00\x05\x73\x74\x75\x70\x30\
            \x0C\x12\x28\x09\x28\x2C\x00\x00\x00\x05\x73\x74\x75\x70\x31\x0C\
            \x13\x28\x09\x28\x08\x00\x00\x00\x05\x73\x74\x75\x70\x32\x00";
        let mut data = Cursor::new(&bytes);
        let lnk = OBJ::read(&mut data).unwrap();

        eprintln!("obj: {:?}", lnk);
    }

    #[test]
    fn test_section() {
        let bytes = b"\x3A\x00\x00\x26\x00\x00\x00\x09\x00";
        let mut data = Cursor::new(&bytes);
        let _ = Section::read(&mut data).unwrap();
    }

    #[test]
    fn test_expression() {
        // ExpressionDefinition::{ // 0x0A
        //   tag: 0x52,            // 0x52 (82)
        //   offset: 8,            // 0x0800 (little endian)
        //   expression: (
        //     sectstart(0x280c)   // 0x0C0C28
        //   )
        // }
        let bytes = b"\x0A\x52\x08\x00\x0C\x0C\x28";
        let mut data = Cursor::new(&bytes);
        let _ = Section::read(&mut data).unwrap();

        let bytes = b"\x0A\x52\x10\x00\x16\x0D\x28";
        let mut data = Cursor::new(&bytes);
        let _ = Section::read(&mut data).unwrap();

        let bytes = b"\x0A\x52\x10\x00\x16\x0D\x28\x04\x04\x00\x00\x00\x00\x00\x00";
        let mut data = Cursor::new(&bytes);
        let _ = Section::read(&mut data).unwrap();

        let bytes = b"\x0A\x52\xD0\x00\x32\x00\x04\x00\x00\x00\x2E\x0C\xFA\x62\x16\xFA\x62";
        let mut data = Cursor::new(&bytes);
        let _ = Section::read(&mut data).unwrap();
    }

    #[test]
    fn test_function_start() {
        let bytes = b"\
            \x4A\x7C\x55\xB4\x05\x00\x00\xA7\x59\x00\x00\x00\x00\x1D\x00\x20\
            \x00\x00\x00\x1F\x00\x00\x00\x03\x80\xF8\xFF\xFF\xFF\x06\x63\x61\
            \x6C\x6C\x6F\x63\x4C"
            .to_vec();

        let mut data = Cursor::new(&bytes);
        let _ = Section::read(&mut data).unwrap();

        let bytes = b"\x0A\x52\x10\x00\x16\x0D\x28".to_vec();
        let mut data = Cursor::new(&bytes);
        let _ = Section::read(&mut data).unwrap();
    }

    #[test]
    fn test_def2() {
        let bytes = b"\
            \x54\x00\x00\x04\x00\x00\x00\x66\x00\x00\x00\x04\x00\x00\x00\x00\
            \x00\x08\x5F\x70\x68\x79\x73\x61\x64\x72\x04\x2E\x65\x6F\x73";

        let mut data = Cursor::new(&bytes);
        let section = Section::read(&mut data).unwrap();

        let Section::Def2(def2) = section else {
            panic!("expected a def2");
        };

        assert_eq!(def2.section, 0);
        assert_eq!(def2.value, 4);
        assert_eq!(def2.class, 102);
        assert_eq!(def2.def_type, 0);
        assert_eq!(def2.size, 4);
        // assert_eq!(def2.dims, Dim::None);
        assert_eq!(def2.tag(), "_physadr");
        assert_eq!(def2.name(), ".eos");
    }

    #[test]
    fn test_libsn_sat() {
        let bytes =
b"\x68\x00\x2F\x86\x2F\x96\x2F\xA6\x2F\xB6\x2F\xC6\x2F\xD6\x2F\xE6\x4F\x22\x6E\xF3\x6D\x43\x6B\x53\x69\x63\x29\x98\x8D\x16\xEA\x00\xDC\x00\x39\xC6\x8F\x01\x68\x93\x68\xC3\x66\x83\x65\xB3\xD1\x00\x41\x0B\x64\xD3\x88\xFF\x8F\x02\x3A\x0C\xA0\x08\xE0\xFF\x3B\x0C\x30\x83\x8F\x03\x39\x08\x29\x98\x8F\xEC\x39\xC6\x60\xA3\x6F\xE3\x4F\x26\x6E\xF6\x6D\xF6\x6C\xF6\x6B\xF6\x6A\xF6\x69\xF6\x00\x0B\x68\xF6\x00\x00\x80\x00\x00\x00\x00\x00";

        let mut data = Cursor::new(&bytes);
        let code = Code::read(&mut data).unwrap();
        assert_eq!(bytes.len(), 106);
        assert_eq!(code.size, 104);
        assert_eq!(code.code.len(), 104);
        assert_eq!(code.code, bytes[2..]);

        let bytes = b"\x0A\x0A\x1F\x00\x4A\x00\x02\x00\x00\x00\x2E\x34\x00\xFC\xFF\xFF\xFF\x2C\x04\x01\x00\x00\x22\x00\x00\x00\x2C\x04\x01\x00\x00\x60\x00\x00\x00";
        let mut data = Cursor::new(&bytes);
        let section = Section::read(&mut data).unwrap();
        assert_eq!(section.to_string(), "10 : Patch type 10 at offset 1f with ($2-arshift_chk-(($fffffffc&(sectbase(1)+$22))-(sectbase(1)+$60)))");

        println!("section: {section}");

        let bytes = b"\x4C\x4E\x4B\x02\x2E\x08\x14\x0B\x33\x80\x03\x62\x73\x73\x10\x0C\x33\x0B\x33\x08\x06\x62\x73\x73\x65\x6E\x64\x06\x0C\x33\x0C\x0A\x33\x0C\x33\x00\x00\x00\x00\x03\x65\x6E\x64\x00";
        let mut data = Cursor::new(&bytes);
        let _ = OBJ::read(&mut data).unwrap();
    }
}
