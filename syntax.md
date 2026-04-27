# Unified BLIF Syntax Reference

This document catalogs every known BLIF (Berkeley Logic Interchange Format)
keyword, directive, and extension found in the wild. It is intended to serve as
the single source of truth for the `turbo-blif` parser.

The format is organized by origin. When multiple tools use the same keyword with
different semantics, all variants are documented.

---

## Legend

- **Origin:** Which tool / specification introduced or commonly emits the
  directive.
- **Usage:** Who generates or consumes files containing this directive.
- **Syntax:** The exact grammar as observed in source code or official docs.
- **Notes:** Quirks, optional arguments, and deviations from the original 1992
  spec.

---

## 1. Core BLIF (Berkeley, 1992)

Reference: *Berkeley Logic Interchange Format (BLIF)*, UC Berkeley, July 28,
1992.

### 1.1 Model Declaration

```
.model <name>
.inputs  <signal> ...
.outputs <signal> ...
.clock   <signal> ...
.end
```

| Origin | Original BLIF spec |
|--------|-------------------|
| Usage  | SIS, ABC, Yosys, VTR, custom academic tools |
| Notes  | `.model`, `.inputs`, `.outputs`, `.clock`, and `.end` are all **optional** per the spec. If `.model` is omitted, the filename is used as the model name. If `.end` is omitted, end-of-file or the next `.model` implies it. Multiple `.inputs`, `.outputs`, and `.clock` lines are allowed and concatenate. |

---

### 1.2 Logic Gate (Truth Table)

```
.names <in1> <in2> ... <out>
<cube> <out_val>
```

| Origin | Original BLIF spec |
|--------|-------------------|
| Usage  | Universal |
| Notes  | `<cube>` uses characters `0`, `1`, `-`. Rows are ORed together; elements within a row are ANDed. `0` = complemented, `1` = uncomplemented, `-` = don't care. Output column is `0`, `1`, `x`, or `n` (ABC allows `x`/`n` in mapped networks). To assign constant `0` to a signal: `.names <sig>` with no cube lines. To assign constant `1`: `.names <sig>` followed by `1`. |

#### `.cover` (alternative)

```
.cover <nin> <nout> <nterms>
<input-list> <output>
<cube-lines>
```

* `<nin>` – number of inputs.
* `<nout>` – must be `1`.
* `<nterms>` – number of product terms that follow.
* The remaining lines are identical to the `.names` cover.

> **Origin:** SIS. Emitted by `write_blif -n` in some versions.

---

### 1.3 External Don't Cares

```
.exdc
.names <in-1> ... <in-n> <output>
<single-output-cover>
...
.end
```

| Origin | Original BLIF spec |
|--------|-------------------|
| Usage  | SIS |
| Notes  | Must appear at the end of a model, before `.end`. The `.names` constructs inside apply to the external don't-care network. Each `<output>` must be a primary output of the main model, and the `.exdc` network can only refer to primary inputs of the main model. Hierarchical specification is not supported. |

---

### 1.4 Generic Latch

```
.latch <input> <output> [<type> <control>] [<init-val>]
```

| Origin | Original BLIF spec |
|--------|-------------------|
| Usage  | Universal |
| Syntax | Types: `fe` (falling edge), `re` (rising edge), `ah` (active high), `al` (active low), `as` (asynchronous). Control may be a signal name or `NIL`. Init: `0`, `1`, `2` (don't care), `3` (unknown). |
| Notes  | The 1992 BLIF spec lists the type `asg` rather than `as`; SIS and ABC use `as`. **ABC quirk:** The standard ABC reader (`ioReadBlif.c`) ignores type, control, and any intermediate tokens; it only reads input, output, and the **last** token as init value (`0`, `1`, `2`). `turbo-blif` parses the full syntax. |

---

### 1.5 Library Gate

```
.gate <gate-name> <formal1>=<actual1> <formal2>=<actual2> ...
```

| Origin | Original BLIF spec |
|--------|-------------------|
| Usage  | SIS, ABC, Yosys |
| Notes  | All formal parameters must be specified. The single output must be the last one in the list. ABC reorders pins to match the Genlib gate definition. |

---

### 1.6 Library Latch

```
.mlatch <gate-name> <formal1>=<actual1> ... <control> [<init-val>]
```

| Origin | Original BLIF spec |
|--------|-------------------|
| Usage  | SIS |
| Notes  | Similar to `.gate` but for technology-dependent latches. Init value semantics identical to `.latch`. |

---

### 1.7 Subcircuit Reference

```
.subckt <model-name> <formal1>=<actual1> ...
```

| Origin | Original BLIF spec |
|--------|-------------------|
| Usage  | Universal |
| Notes  | Model need not be previously defined in the same file (may come from `.search`). Unlike `.gate`, multiple outputs are allowed. Formals may appear in any order. |

**Alias:** ABC's standard reader also historically supported `.subcircuit` (currently commented out in `ioReadBlif.c`), while the MV reader uses `.subckt`.

---

### 1.8 Subfile Reference

```
.search <file-name>
```

| Origin | Original BLIF spec |
|--------|-------------------|
| Usage  | SIS, `turbo-blif` |
| Notes  | Can appear inside or outside a `.model`. Directs the reader to pause, read the named file as an independent self-contained BLIF, then resume. Nested searches are allowed. |

---

### 1.9 Finite State Machine

```
.start_kiss
.i <num-inputs>
.o <num-outputs>
[.p <num-terms>]
[.s <num-states>]
[.r <reset-state>]
<input> <current-state> <next-state> <output>
...
.end_kiss
[.latch_order <sig1> <sig2> ...]
[.code <state> <bits>]
```

| Origin | Original BLIF spec |
|--------|-------------------|
| Usage  | SIS |
| Notes  | `.p` and `.s` are optional. `.r` defaults to the first state encountered. `.latch_order` and `.code` map FSM state variables to physical latches when both logic and FSM descriptions are present. |

---

### 1.10 Clock Constraints

```
.cycle <cycle-time>
.clock_event <percent> <event> [<event> ...]
```

Where `<event>` is:

```
r'<clock-name>
f'<clock-name>
( f'<clock-name> <before> <after> )
```

| Origin | Original BLIF spec |
|--------|-------------------|
| Usage  | SIS (simulation) |
| Notes  | Multiple events in one `.clock_event` line are "linked" (changing one should change all). Parenthesized form adds skew bounds. |

---
### 1.11 Delay and Timing Constraints

These are embedded as dot-commands inside the model.

| Command | Arguments | Meaning |
|---|---|---|
| `.area` | `<area>` | Area of the model. |
| `.delay` | `<in-name> <phase> <load> <max-load> <brise> <drise> <bfall> <dfall>` | Delay parameters for a primary input. `<phase>` is `INV`, `NONINV`, or `UNKNOWN`. |
| `.wire_load_slope` | `<load>` | Wire-load slope for the model. |
| `.wire` | `<load> <load> ...` | Wire loads (list of floats). |
| `.input_arrival` | `<in-name> <rise> <fall> [<before-after> <event>]` | Input arrival time. Optional `b` (before) or `a` (after) a clock edge. |
| `.output_required` | `<out-name> <rise> <fall> [<before-after> <event>]` | Output required time. |
| `.default_input_arrival` | `<rise> <fall>` | Default arrival for all inputs. |
| `.default_output_required` | `<rise> <fall>` | Default required time for all outputs. |
| `.input_drive` | `<in-name> <rise> <fall>` | Input drive strength. |
| `.default_input_drive` | `<rise> <fall>` | Default drive for all inputs. |
| `.output_load` | `<out-name> <load>` (SIS) or `<out-name> <rise> <fall>` (ABC) | Output load. **SIS uses a single load value; ABC uses rise/fall pair.** |
| `.default_output_load` | `<load>` (SIS) or `<rise> <fall>` (ABC) | Default load for all outputs. |
| `.max_input_load` | `<in-name> <load>` | Maximum load an input can drive. |
| `.default_max_input_load` | `<load>` | Default max input load. |

| Origin | Original BLIF spec |
|--------|-------------------|
| Usage  | SIS, ABC |
| Notes  | The 1992 spec documents optional `[<before-after> <event>]` suffixes for `.input_arrival` and `.output_required`. **ABC does NOT implement these suffixes**; it strictly uses the 3-arg and 2-arg forms. ABC stores timing internally as floats. `.area` is emitted by ABC but treated as a generic model attribute. Timing values are unitless; they must be consistent with the `.cycle` value. |

---

## 2. ABC Extensions

Reference: ABC source (`ioReadBlif.c`, `ioWriteBlif.c`, `ioReadBlifMv.c`),
*Extended BLIF Specification with Black and White Boxes* (Mishchenko, 2009).

### 2.1 Black Box Statement

```
.blackbox
```

| Origin | ABC |
|--------|-----|
| Usage  | ABC write-blif, Yosys `write_blif -blackbox` |
| Notes  | Appears inside a model with no logic. Marks the model as a black box. No arguments. |

---

### 2.2 AND Gate Delay

```
.and_gate_delay <delay>
```

| Origin | ABC |
|--------|-----|
| Usage  | ABC `write_blif` |
| Notes  | Exactly one float argument. Sets the default AND-gate delay for the network. |

---

### 2.3 Box Attributes (Black / White Boxes)

```
.attrib white box comb
.attrib white box seq
.attrib black box comb
.attrib black box seq
.attrib white box comb keep
...
.no_merge <signal> ...
```

| Origin | ABC Extended BLIF (boxes paper) |
|--------|--------------------------------|
| Usage  | ABC |
| Notes  | Describes hierarchical boxes. `.attrib` lines specify presence type (`white`/`black`), logic type (`comb`/`seq`), and persistence (`sweep`/`keep`). `.no_merge` prevents output collapsing on a per-signal basis. |

---

### 2.4 Box Timing

```
.delay 1
.delay <signal> <delay>
.delay <in-sig> <out-sig> <delay>
.input_required <sig> <time>
.output_arrival <sig> <time>
```

| Origin | ABC Extended BLIF |
|--------|-------------------|
| Usage  | ABC |
| Notes  | `.delay` has three overloads: global delay, per-signal delay, or per-input-output-pair delay. Later `.delay` overrides earlier ones. Sequential boxes use `.input_required` / `.output_arrival` instead of `.delay`. |

---

### 2.5 Extended Flip-Flop (`.flop`)

```
.flop [D=<in>] [Q=<out>] [C=<clk>] [S=<set>] [R=<reset>] [E=<enable>] [async] [negedge] [init=<val>]
```

| Origin | ABC Extended BLIF |
|--------|-------------------|
| Usage  | ABC |
| Notes  | Arguments can appear in any order. `init=` follows the same encoding as `.latch` (`0`, `1`, `2`). `async` means asynchronous set/reset. `negedge` means negative-edge triggered. ABC's MV reader converts `.flop` into a `.latch` plus surrounding logic, so some information may be lost during round-trip. |

---

### 2.6 Register Classes in `.latch`

```
.latch <in> <out> <type> <control> <init> <class>
```

| Origin | ABC Extended BLIF |
|--------|-------------------|
| Usage  | ABC |
| Notes  | An integer `<class>` may follow the init value to distinguish multiple clock domains or register types during sequential synthesis. Standard BLIF parsers that do not support classes will see the integer as an extra token. |

---

## 3. EBLIF / VTR / Yosys Extensions

Reference: Yosys `write_blif` documentation, VTR file format pages,
`verilog-to-routing` discussions.

### 3.1 Cell Name
### 3.6 Cell Name on `.names`

```
.cname <name>
```

| Origin | EBLIF (Yosys / VTR) |
|--------|---------------------|
| Usage  | Yosys `write_blif -iname`, VPR |
| Notes  | Attaches a cell name to the preceding `.names` statement. Emitted **after** the truth table. Yosys `-cname` emits `.cname` for `.subckt`/`.gate`; `-iname` additionally emits it for `.names` blocks. |
| Notes  | Attaches a cell name to the preceding `.subckt`, `.gate`, or `.names` (when `-iname` is used). |

---

### 3.2 Attributes
### 3.7 Attributes on `.names`

```
.attr <key> <value>
```

| Origin | EBLIF (Yosys / VTR) |
|--------|---------------------|
| Usage  | Yosys `write_blif -iattr`, VPR |
| Notes  | Attaches an attribute to the preceding `.names` statement. Emitted **after** the truth table. The `<value>` may be an empty string (VTR parser accepts `.attr <key>` with no value). |
| Notes  | `<value>` is typically a quoted string (e.g. `"src"`). Attaches to the last cell or `.names`. |

---

### 3.3 Parameters

```
.param <key> <value>
```

| Origin | EBLIF (Yosys / VTR) |
|--------|---------------------|
| Usage  | Yosys `write_blif -param`, VPR |
| Notes  | `<value>` is often a binary-encoded integer (e.g. `00000000000000000000000000000001`). Attaches to the last cell or `.names`. |

---

### 3.4 Connection (Wire)

```
.conn <from> <to>
```

| Origin | EBLIF (Yosys / VTR) |
|--------|---------------------|
| Usage  | Yosys `write_blif -conn` |
| Notes  | Direct wire connection without a buffer. Also emitted as `.barbuff` in some Yosys versions (synonym). |

---

### 3.5 Buffer Alias

```
.barbuff <from> <to>
```

| Origin | ABC |
|--------|-----|
| Usage  | ABC (mapped netlists) |
| Notes  | Semantically identical to `.conn`. ABC emits `.barbuf` (one `f`) for "barrier buffers" in mapped networks. `turbo-blif` accepts `.barbuff` as a synonym for compatibility. |

---

## 4. BLIF-MV Extensions

Reference: ABC `ioReadBlifMv.c`, `ioWriteBlifMv.c`.

### 4.1 Multi-Valued Variable

```
.mv <var> [<var> ...] <nvalues> [<val-name> ...]
```

| Origin | BLIF-MV (Vis / MV-SIS) |
|--------|------------------------|
| Usage  | ABC (MV reader) |
| Notes  | Declares a variable with more than two values. Symbolic value names are optional. |

### 4.8 LTL Properties

```
.ltlformula "<LTL string>"
```

| Origin | BLIF-MV |
|--------|---------|
| Usage  | ABC |
| Notes  | Stores an LTL property string verbatim. |

---


### 4.2 Multi-Valued Table

```
.table <in1> <in2> ... -> <out1> <out2> ...
```

| Origin | BLIF-MV |
|--------|---------|
| Usage  | MVSIS, ABC |
| Notes  | Multi-valued counterpart to `.names`. Supports `->` to separate inputs from outputs. MVSIS emits `.table` when `FileType == IO_FILE_BLIF_MV`. |

---

### 4.3 Short (Buffer)

```
.short <in> <out>
```

| Origin | BLIF-MV / ABC |
|--------|---------------|
| Usage  | ABC |
| Notes  | ABC replaces trivial `.names` buffers with `.short` when writing mapped netlists. Semantically a buffer. |

---

### 4.4 Specification

```
.spec <file-name>
```

| Origin | BLIF-MV |
|--------|---------|
| Usage  | MVSIS |
| Notes  | Appears immediately after `.model` in BLIF-MV output. References an external specification file (e.g. CTL / LTL properties). |

---

### 4.5 Constraints

```
.constraint <signal> ...
```

| Origin | BLIF-MV |
|--------|---------|
| Usage  | ABC |
| Notes  | Declares constraint signals (pseudo-outputs). |

---

### 4.6 Reset

```
.reset <signal>
<value>
```

| Origin | BLIF-MV |
|--------|---------|
| Usage  | MVSIS, ABC |
| Notes  | Appears after a `.latch` in BLIF-MV to specify reset value. MVSIS emits `.reset <sig>` followed by the value on the next line. For multi-valued (`BLIF_MVS`), the line contains one digit per value. |

---

### 4.7 One-Hot

```
.onehot <signal> ...
```

| Origin | BLIF-MV |
|--------|---------|
| Usage  | ABC |
| Notes  | Declares a group of registers that are one-hot encoded. |

---

## 5. Known Quirks & Edge Cases

1. **Line continuation:** Backslash `\` as the last non-comment character of a
   line concatenates the next line. No whitespace should follow the `\`.
2. **Comments:** Hash `#` starts a comment extending to end-of-line. `#` cannot
   appear inside signal names.
3. **`.inputs` / `.outputs` inference:** If omitted, inputs are inferred from
   signals that are not outputs of any other block, and outputs from signals
   that are not inputs to any other block.
4. **`.latch` init:** ABC's standard reader only looks at the **last** token
   on the line as the init value; it ignores the type and clock fields.
5. **`.names` output column in ABC:** `x` and `n` are accepted in mapped
   networks (in addition to `0` and `1`).
6. **Subcircuit instance names:** BLIF-MV allows `model|instance` syntax in
   `.subckt` (instance name separated by `|`).
7. **Signal name escaping:** Yosys replaces `#`, `=`, `<`, `>` with `?` in
   signal names when writing BLIF.
8. **Spare `.end`:** ABC readers append `\n.end\n` to the file buffer because
   some benchmarks omit the final `.end`.

---

## 6. Remaining Gaps (To Be Refined)

The following directives and behaviors are documented but **not yet fully
implemented** in `turbo-blif`. They are listed here for future work.

| # | Directive / Feature | Origin | Status |
|---|---------------------|--------|--------|
| 1 | `.default_input_arrival` | Core BLIF, ABC | AST exists; parser case missing |
| 2 | `.default_output_required` | Core BLIF, ABC | AST exists; parser case missing |
| 3 | `.and_gate_delay` | ABC | Parser case missing |
| 4 | `.input_arrival <sig> <rise> <fall>` | Core BLIF | Not in AST or parser |
| 5 | `.output_required <sig> <rise> <fall>` | Core BLIF | Not in AST or parser |
| 6 | `.input_drive <sig> <rise> <fall>` | Core BLIF | Not in AST or parser |
| 7 | `.default_input_drive <rise> <fall>` | Core BLIF | Not in AST or parser |
| 8 | `.output_load` (SIS: 1 load; ABC: rise/fall) | Core BLIF, ABC | Not in AST or parser |
| 9 | `.default_output_load` (SIS: 1 load; ABC: rise/fall) | Core BLIF, ABC | Not in AST or parser |
| 10 | `.blackbox` | ABC, Yosys | Parser case missing |
| 11 | `.attrib` / `.no_merge` | ABC Extended BLIF | Not implemented |
| 12 | `.flop` | ABC Extended BLIF | Not implemented |
| 13 | `.input_required` (box timing) | ABC Extended BLIF | Not implemented |
| 14 | `.output_arrival` (box timing) | ABC Extended BLIF | Not implemented |
| 15 | `.subcircuit` alias | ABC | Not implemented |
| 16 | `.table` | BLIF-MV | Not implemented |
| 17 | `.mv` | BLIF-MV | Not implemented |
| 18 | `.short` | BLIF-MV / ABC | Not implemented |
| 19 | `.constraint` | BLIF-MV | Not implemented |
| 20 | `.reset` | BLIF-MV | Not implemented |
| 21 | `.onehot` | BLIF-MV | Not implemented |
| 22 | `.ltlformula` | BLIF-MV | Not implemented |
| 23 | `.subckt` instance name (`model\|instance`) | BLIF-MV | Not implemented |
| 24 | `.delay` per-signal / per-pair overloads | ABC Extended BLIF | Not implemented |
| 25 | `.latch` register class integer | ABC Extended BLIF | Not implemented |
| 26 | `.names` output chars `x`, `n` | ABC quirk | Not implemented |
| 27 | `.cover` alternative header | SIS | Not implemented |
| 28 | `.spec` | BLIF-MV | Not implemented |
| 29 | `.gateinit` | Yosys | Not implemented |

---

## 7. References

1. *Berkeley Logic Interchange Format (BLIF)*, UC Berkeley, July 28, 1992.
2. Mishchenko, A. *Extended BLIF Specification with Black and White Boxes*,
   UC Berkeley, October 2009.
3. ABC source: `ioReadBlif.c`, `ioWriteBlif.c`, `ioReadBlifMv.c`,
   `ioWriteBlifMv.c`, `ioReadBlifAig.c`.
4. Yosys documentation: `help write_blif`.
5. VTR / EBLIF discussions: `verilog-to-routing` GitHub issues #281, #307,
   Yosys PR #596.
6. SIS documentation: `file-formats/BLIF.md`, `file-formats/KISS.md`,
   `file-formats/KISS2.md`, `file-formats/PLA.md`, `file-formats/SLIF.md`,
   `file-formats/EQN.md`, `file-formats/GENLIB.md`, `file-formats/ASTG.md`,
   `file-formats/OCT.md`, `file-formats/BDNET.md`, `file-formats/PDS.md`,
   `file-formats/VHDL-VST.md` — recovered SIS file-format reference manual.
7. Yosys source: `backends/blif/blif.cc` — Yosys `write_blif` emitter.
8. VTR / libblifparse: `blif_lexer.l`, `blif_parser.y` — EBLIF grammar.
9. MVSIS source: `src/base/io/ioWrite.c`, `src/base/io/io.c`,
   `src/graph/sh/shResyn.c` — MVSIS BLIF / BLIF-MV writers.