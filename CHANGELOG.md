Change Log
==========

0.4.0 - December 18, 2025
-------------------------

* `SectionSwitch` has been removed and replaced with the `section_id`
  being stored directly in the `Section` enum.
* `cputype::MIPS_R300GTE` [sic] was renamed `cputype::MIPS_R3000`.
* `Expression::SectOf` was renamed to `Expression::SectionOffset`.
* `Expression::GroupOf` was renamed to `Expression::GroupOffset`.
* `FunctionEnd`, `BlockStart`, and `BlockEnd` structs have been replaced with
  common `SectionOffsetLine` struct.
* `OBJ::sections` is now private. Use `OBJ::sections()` instead.
* Added remaining section types that don't have previously seen examples.
  `Section::ProcedureCall` and `Section::ProcedureDefinition` are incorrect.
* Added `-r`/`--recursive` option for `psyk list` which will print all
  all `OBJ` data for each module in a `LIB`
* File format documentation for all types `LIB` and `OBJ` types.

0.3.1 - December 15, 2025
-------------------------

* Code coverage reporting changes.

0.3.0 - December 11, 2025
-------------------------

* Name change to Psy-K to avoid conflicts with PsyCross

0.2.0 - December 11, 2025
-------------------------

* Adds the ability to parse `psylink` linker scripts

0.1.0 - December 9, 2025
------------------------

* Adds `psylib` and `dumpobj` binaries which compatible output to the original
* `psyx` binary supports LIB mutation sub commands (add, delete, update, etc.)
* Adds the ability to create `LIB`s programmatically
* Additional public interfaces to `LIB` and `OBJ` data structures

0.0.0 - December 2, 2025
------------------------

* Initial release with support for Genesis, Saturn, and PS1 LIB and OBJ files
