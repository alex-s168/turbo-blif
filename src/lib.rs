//! A high-performance BLIF (Berkeley Logic Interchange Format) parser.
//!
//! This crate provides a parser for the standard BLIF format, as well as
//! several extensions including BLIF-MV, ABC Extended BLIF, and Yosys-specific
//! extensions. It exposes a consumer-trait-based API that allows users to
//! process BLIF files without necessarily materialising the full AST.
//!
//! # Overview
//!
//! The crate is organised around a set of **consumer traits**:
//!
//! - [`ModelConsumer`] — receives notifications when a new model begins/ends
//!   and when a `.search` directive is encountered.
//! - [`CommandConsumer`] — receives per-command callbacks within a model
//!   (gates, flip-flops, sub-circuits, delay constraints, BLIF-MV extensions, etc.).
//! - [`GateLutConsumer`] — receives truth-table rows for a `.names` gate.
//! - [`FSMConsumer`] — receives transitions for a finite state machine
//!   (`.start_kiss` / `.end_kiss` block).
//!
//! For convenience, a ready-to-use [`ast`] module provides concrete AST types
//! that implement all of the consumer traits, so you can parse straight into an AST:
//!
//! ```rust
//! use turbo_blif::ast::parse_str_blif_to_ast;
//! let blif = parse_str_blif_to_ast("test", ".model simple\n.inputs a\n.outputs b\n.names a b\n1 1\n.end\n").unwrap();
//! ```
//!
//! # BLIF format support
//!
//! | Feature | Status |
//! |---|---|
//! | Standard BLIF (.model, .inputs, .outputs, .names, .latch, .gate, .subckt, .search, .start_kiss/.end_kiss, .delay, .attr, .param, .cname, .conn, .area, .clock, .clock_event, .cycle, .wire_load_slope, .wire, .input_arrival, .output_required, .default_input_arrival, .default_output_required, .input_drive, .default_input_drive, .output_load, .default_output_load, .max_input_load, .default_max_input_load, .and_gate_delay, .input_required, .output_arrival, .exdc, .blackbox, .mlatch) | Supported |
//! | BLIF-MV (.constraint, .onehot, .reset, .ltlformula, .spec, .gateinit, .mv, .short, .table) | Supported |
//! | ABC Extended BLIF (.flop, .subcircuit, .cover) | Supported |
//! | Yosys extensions (.attr, .param, .cname) | Supported |
//!
//! # Error handling
//!
//! The parser reports errors through [`BlifParserError`]. When combined with
//! I/O errors via the [`ast::FullBlifErr`] wrapper, callers get a uniform
//! error type for all parsing operations.

use smallvec::SmallVec;
use std::{iter::Peekable, str::FromStr};

pub mod ast;
pub mod writer;

/// A fixed-capacity, inline-or-heap string used throughout the crate.
///
/// `N` is the inline capacity in bytes. For example, `Str<16>` can store up
/// to 15 bytes inline (plus a null terminator) before spilling to the heap.
///
/// This is backed by [`smallstr::SmallString`].
pub type Str<const N: usize> = smallstr::SmallString<[u8; N]>;

/// Metadata for a single logic gate (`.names` or `.table` declaration).
#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct GateMeta {
    /// Input signal names.
    pub inputs: Vec<Str<16>>,
    /// Output signal name.
    pub output: Str<16>,
    /// Whether this gate is part of an `.exdc` (external don't-care) block.
    pub external_dc: bool,
}

/// Consumer for the truth-table rows of a single gate.
///
/// After a `.names` or `.table` line is encountered, the parser calls
/// [`entry`](Self::entry) once for each row of the truth table.
pub trait GateLutConsumer {
    /// Feed one row of the truth table.
    ///
    /// * `ins` — the input pattern (each element is 0, 1, or don't-care/-).
    /// * `out` — the output value:
    ///   - `Some(true)` — `1`
    ///   - `Some(false)` — `0`
    ///   - `None` — don't care (`x` or `n` in the output column)
    fn entry(&mut self, ins: SmallVec<[Tristate; 8]>, out: Option<bool>);
}

/// A flip-flop or latch (`.latch` or `.flop` declaration).
#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct FlipFlop {
    /// The flip-flop type, if explicitly declared.
    ///
    /// If `None`, this is a generic D flip-flop.
    pub ty: Option<FlipFlopType>,
    /// Input signal name.
    pub input: Str<16>,
    /// Output signal name.
    pub output: Str<16>,
    /// Optional clock signal name.
    ///
    /// If a latch does not have a controlling clock specified, it is assumed
    /// that it is actually controlled by a single global clock. The behaviour
    /// of this global clock may be interpreted differently by the various
    /// algorithms that may manipulate the model after the model has been
    /// read in.
    pub clock: Option<Str<16>>,
    /// Initialisation value of the flip-flop.
    pub init: FlipFlopInit,
}

/// A reference to a technology-library gate (`.gate` declaration).
#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct LibGate {
    /// Name of the gate in the technology library.
    pub name: Str<16>,
    /// Mappings of wires from technology library gate to the rest of the circuit.
    ///
    /// From the spec:
    /// > All of the formal parameters of `name` must be specified in the
    /// > formal-actual-list and the single output of `name` must be the
    /// > last one in the list.
    pub maps: Vec<(Str<16>, Str<16>)>,
}

/// A reference to a technology-library flip-flop (`.mlatch` declaration).
#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct LibFlipFlop {
    /// Name of the gate in the technology library.
    pub name: Str<16>,
    /// Mappings of wires from technology library gate to the rest of the circuit.
    ///
    /// From the spec:
    /// > All of the formal parameters of `name` must be specified in the
    /// > formal-actual-list and the single output of `name` must be the
    /// > last one in the list.
    pub maps: Vec<(Str<16>, Str<16>)>,
    /// Optional clock signal name.
    ///
    /// If a latch does not have a controlling clock specified, it is assumed
    /// that it is actually controlled by a single global clock. The behaviour
    /// of this global clock may be interpreted differently by the various
    /// algorithms that may manipulate the model after the model has been
    /// read in.
    pub clock: Option<Str<16>>,
    /// Initialisation value of the flip-flop.
    pub init: FlipFlopInit,
}

/// Consumer for FSM transitions inside a `.start_kiss` / `.end_kiss` block.
pub trait FSMConsumer {
    /// Add one transition to the FSM.
    fn add_transition(&mut self, transition: FSMTransition);
}

/// A cell attribute attached to the most recently declared gate / FSM /
/// flip-flop / library gate / sub-circuit.
///
/// These are non-standard extensions emitted by tools such as Yosys or ABC.
#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub enum CellAttr<'a> {
    /// Set the name of the current cell (`.cname`).
    ///
    /// Non-standard; emitted by Yosys.
    CellName(&'a str),

    /// An arbitrary key-value attribute (`.attr`).
    ///
    /// Non-standard; possibly emitted by Yosys.
    ///
    /// Example: `Attr { key: "src", val: "\"some/file.v:320.20-320.28\"" }`
    Attr {
        /// Attribute key.
        key: &'a str,
        /// Attribute value.
        val: &'a str,
    },

    /// A parameter assignment (`.param`).
    ///
    /// Non-standard; emitted by Yosys.
    ///
    /// Example: `Param { key: "A_WIDTH", val: "00000000000000000000000000000001" }`
    Param {
        /// Parameter key.
        key: &'a str,
        /// Parameter value (string representation).
        val: &'a str,
    },
}

/// Consumer for all commands that can appear inside a `.model` block.
///
/// Implement this trait to drive your own data structure from a BLIF file
/// without necessarily building the full AST.
pub trait CommandConsumer {
    /// The concrete type used to collect gate truth-table rows.
    type Gate: GateLutConsumer;
    /// The concrete type used to collect FSM transitions.
    type FSM: FSMConsumer;

    /// Create a new gate container from its metadata.
    ///
    /// The returned object will receive [`GateLutConsumer::entry`] calls for
    /// each truth-table row, then [`gate_done`](Self::gate_done) is called.
    fn gate(&self, gate: GateMeta) -> Self::Gate;
    /// Finalise a gate whose truth-table rows have all been fed in.
    fn gate_done(&mut self, gate: Self::Gate);

    /// Create a new FSM container.
    ///
    /// The returned object will receive
    /// [`FSMConsumer::add_transition`] calls, then
    /// [`fsm_done`](Self::fsm_done) is called.
    fn fsm(&self, inputs: usize, outputs: usize, reset_state: Option<&str>) -> Self::FSM;
    /// Finalise the FSM with optional latch ordering and state encoding.
    fn fsm_done(
        &mut self,
        fsm: Self::FSM,
        physical_latch_order: Option<Vec<String>>,
        state_assignments: Option<Vec<(String, SmallVec<[bool; 8]>)>>,
    );

    /// Process a `.latch` or `.flop` declaration.
    fn ff(&mut self, ff: FlipFlop);
    /// Process a `.gate` (library gate) declaration.
    fn lib_gate(&mut self, gate: LibGate);
    /// Process a `.mlatch` (library flip-flop) declaration.
    fn lib_ff(&mut self, ff: LibFlipFlop);
    /// Instantiate a sub-model (`.subckt`).
    ///
    /// Copies the whole circuit of the referenced model and maps the
    /// inputs / outputs / clocks according to `map`.
    ///
    /// `instance_name` is an optional instance name from the
    /// `model|instance` BLIF-MV syntax.
    fn sub_model(&mut self, model: &str, map: Vec<(Str<16>, Str<16>)>, instance_name: Option<&str>);

    /// Attach a cell attribute (`.cname`, `.attr`, or `.param`) to the most
    /// recently declared gate / FSM / flip-flop / library gate / sub-circuit.
    fn attr(&mut self, attr: CellAttr);

    /// Process a `.conn` (or `.barbuff`, `.short`) direct connection.
    ///
    /// Non-standard; connects `from` to `to` directly.
    fn connect(&mut self, from: &str, to: &str);

    /// Process an `.area` attribute.
    fn set_area(&mut self, area: f64);
    /// Process a delay constraint (`.delay`, `.input_arrival`, etc.).
    fn model_delay_constraint(&mut self, constraint: ModelDelayConstraint);

    /// Set the cycle time (`.cycle`).
    fn set_cycle_time(&mut self, cycle_time: f32);
    /// Process a `.clock_event` declaration.
    fn clock_events(&mut self, events: ClockEvents);

    /// BLIF-MV: `.constraint <signal> ...`
    fn constraint(&mut self, signals: &[Str<16>]) {
        let _ = signals;
    }
    /// BLIF-MV: `.onehot <signal> ...`
    fn onehot(&mut self, signals: &[Str<16>]) {
        let _ = signals;
    }
    /// BLIF-MV: `.reset <signal> <value>`
    fn reset(&mut self, signal: Str<16>, value: SmallVec<[Tristate; 8]>) {
        let _ = (signal, value);
    }
    /// BLIF-MV: `.ltlformula "<LTL string>"`
    fn ltlformula(&mut self, formula: &str) {
        let _ = formula;
    }
    /// BLIF-MV: `.spec <file-name>`
    fn spec(&mut self, filename: &str) {
        let _ = filename;
    }
    /// BLIF-MV / Yosys: `.gateinit <signal>=<init-val>`
    fn gateinit(&mut self, signal: Str<16>, value: FlipFlopInit) {
        let _ = (signal, value);
    }
    /// BLIF-MV: `.mv <var> ... <nvalues> [<val-name> ...]`
    fn mv(&mut self, variables: Vec<Str<16>>, nvalues: usize, value_names: Vec<String>) {
        let _ = (variables, nvalues, value_names);
    }
}

/// Metadata for a `.model` declaration.
#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct ModelMeta {
    /// The model name.
    pub name: Str<32>,
    /// Declared input signals.
    ///
    /// If `None`, inputs **must** be inferred from the netlist (signals that
    /// are not outputs from other blocks).
    pub inputs: Option<Vec<Str<16>>>,
    /// Declared output signals.
    ///
    /// If `None`, outputs **must** be inferred from the netlist (signals that
    /// are not inputs to other blocks).
    pub outputs: Option<Vec<Str<16>>>,
    /// Declared clock signals.
    pub clocks: Vec<Str<16>>,
}

/// Consumer for top-level BLIF constructs (models and search directives).
pub trait ModelConsumer {
    /// The concrete type used to consume commands within each model.
    type Inner: CommandConsumer;

    /// Called at the start of a `.model` block.
    ///
    /// The returned object will receive per-command callbacks and
    /// then [`model_done`](Self::model_done) is called.
    fn model(&self, meta: ModelMeta) -> Self::Inner;
    /// Called at the end of a `.model` block (i.e. when `.end` is encountered).
    fn model_done(&mut self, model: Self::Inner);

    /// Process a `.search` directive, which tells the parser to look in
    /// an additional BLIF file for more model declarations.
    fn search(&mut self, path: &str);
}

/// The edge type of a clock signal.
#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub enum ClockEdgeKind {
    /// Rising edge (transition from 0 to 1).
    Rise,
    /// Falling edge (transition from 1 to 0).
    Fall,
}

/// A single clock event, specifying when a particular clock edge occurs.
///
/// `before_after` is used to define the "skew" in the clock edges. The unit
/// is to be interpreted by the user. The nominal time is
/// [`ClockEvents::percent`] of the current cycle time.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ClockEvent {
    /// Whether this is a rising or falling edge.
    pub edge: ClockEdgeKind,
    /// The name of the clock signal.
    pub clock_name: Str<16>,
    /// Maximum amount of time before/after the nominal time that the edge
    /// can arrive, represented as `(before, after)`.
    pub before_after: Option<(f32, f32)>,
}

/// A set of clock events with a common nominal-time percentage.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ClockEvents {
    /// The nominal time of these events as a percentage of the cycle time.
    pub percent: f32,
    /// The individual clock events.
    pub events: Vec<ClockEvent>,
}

/// Skip blank lines and comment lines (starting with `#`) in the input.
fn parse_padding(lines: &mut Peekable<impl Iterator<Item = impl AsRef<str>>>) {
    while lines
        .peek()
        .is_some_and(|x| x.as_ref().trim().is_empty() || x.as_ref().trim().starts_with('#'))
    {
        let _ = lines.next();
    }
}

/// Errors that can occur while parsing a BLIF file.
#[derive(Debug)]
pub enum BlifParserError {
    /// An unrecognised keyword was encountered.
    UnknownKw(String),
    /// A command did not have enough arguments.
    MissingArgs,
    /// A command had more arguments than expected.
    TooManyArgs,
    /// The input contained invalid or malformed syntax.
    Invalid,
    /// The input ended unexpectedly (e.g. inside a multi-line construct).
    UnexpectedEnd,
}

impl std::fmt::Display for BlifParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlifParserError::UnknownKw(kw) => write!(f, "unknown keyword: `{kw}`"),
            BlifParserError::MissingArgs => write!(f, "missing arguments"),
            BlifParserError::TooManyArgs => write!(f, "too many arguments"),
            BlifParserError::Invalid => write!(f, "invalid syntax"),
            BlifParserError::UnexpectedEnd => write!(f, "unexpected end of file"),
        }
    }
}

impl std::error::Error for BlifParserError {}

/// A string that is either a direct reference or an owned copy.
///
/// This is used internally to handle multi-line BLIF statements that must
/// be joined before processing.
enum AsRefOrString<T> {
    /// A direct reference (no comment processing needed).
    AsRef(T),
    /// An owned copy (produced after comment stripping or line joining).
    String(String),
}

impl<T: AsRef<str>> AsRef<str> for AsRefOrString<T> {
    fn as_ref(&self) -> &str {
        match self {
            AsRefOrString::AsRef(a) => a.as_ref(),
            AsRefOrString::String(s) => s.as_ref(),
        }
    }
}

/// Strip everything after the first `#` (the comment character).
fn before_cmt(s: &str) -> &str {
    s.split('#').next().unwrap()
}

#[test]
fn test_before_cmt() {
    assert_eq!(before_cmt("aa asa as a # comment"), "aa asa as a ");
}

/// Read the next BLIF statement from the input, joining continuation lines
/// (lines ending with `\`) and stripping comments.
fn next_stmt<S: AsRef<str>>(
    lines: &mut Peekable<impl Iterator<Item = S>>,
) -> Result<Option<AsRefOrString<S>>, BlifParserError> {
    let s_orig = match lines.next() {
        Some(x) => x,
        None => {
            return Ok(None);
        }
    };
    let s = before_cmt(s_orig.as_ref()).trim_end();
    Ok(Some(if s.ends_with('\\') {
        let mut s = s_orig;
        let mut whole = String::new();
        while before_cmt(s.as_ref()).trim_end().ends_with('\\') {
            whole.push_str(before_cmt(s.as_ref()).trim_end().trim_end_matches('\\'));
            s = lines.next().ok_or(BlifParserError::UnexpectedEnd)?;
        }
        whole.push_str(before_cmt(s.as_ref()).trim_end());

        AsRefOrString::String(whole)
    } else {
        if s_orig.as_ref().contains('#') {
            AsRefOrString::String(s.to_string())
        } else {
            AsRefOrString::AsRef(s_orig)
        }
    }))
}

#[test]
fn test_nextstmt_simple() {
    let mut lines = ["this is line 0"].into_iter().peekable();
    assert_eq!(
        next_stmt(&mut lines).unwrap().unwrap().as_ref(),
        "this is line 0"
    );
    assert!(lines.peek().is_none());
}

#[test]
fn test_nextstmt_cmt() {
    let mut lines = ["this is line 0 # but this is a comment"]
        .into_iter()
        .peekable();
    assert_eq!(
        next_stmt(&mut lines).unwrap().unwrap().as_ref(),
        "this is line 0"
    );
    assert!(lines.peek().is_none());
}

#[test]
fn test_nextstmt_simple_one_line() {
    let mut lines = ["this is line 0", "this is line 1"].into_iter().peekable();
    assert_eq!(
        next_stmt(&mut lines).unwrap().unwrap().as_ref(),
        "this is line 0"
    );
    assert!(lines.next().unwrap() == "this is line 1");
}

#[test]
fn test_nextstmt_simple_multiline() {
    let mut lines = ["this is line 0 \\", "this is line 1", "this is line 2"]
        .into_iter()
        .peekable();
    assert_eq!(
        next_stmt(&mut lines).unwrap().unwrap().as_ref(),
        "this is line 0 this is line 1"
    );
    assert!(lines.next().unwrap() == "this is line 2");
}

#[test]
fn test_nextstmt_simple_multiline_cmt() {
    let mut lines = [
        "this is line 0 \\ # comment",
        "this is line 1 # comment",
        "this is line 2",
    ]
    .into_iter()
    .peekable();
    assert_eq!(
        next_stmt(&mut lines).unwrap().unwrap().as_ref(),
        "this is line 0 this is line 1"
    );
    assert!(lines.next().unwrap() == "this is line 2");
}

/// Check whether the next non-padding line starts with the given keyword.
fn is_kw(lines: &mut Peekable<impl Iterator<Item = impl AsRef<str>>>, kw: &str) -> bool {
    lines
        .peek()
        .is_some_and(|x| x.as_ref().split(' ').next().is_some_and(|y| y == kw))
}

/// A three-valued logic type representing a signal level.
///
/// This is used in truth-table rows and FSM transition tables.
#[repr(u8)]
#[derive(Clone, Hash, PartialEq, PartialOrd)]
pub enum Tristate {
    /// Logic 0 (false).
    False = 0,
    /// Logic 1 (true).
    True = 1,
    /// Don't care / ignored (written as `-` in BLIF).
    Ignored,
}

impl std::fmt::Display for Tristate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tristate::False => write!(f, "0"),
            Tristate::True => write!(f, "1"),
            Tristate::Ignored => write!(f, "-"),
        }
    }
}

impl std::fmt::Debug for Tristate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl FromStr for Tristate {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(Self::False),
            "1" => Ok(Self::True),
            "-" => Ok(Self::Ignored),
            _ => Err(()),
        }
    }
}

/// The type/behaviour of a flip-flop or latch.
#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub enum FlipFlopType {
    /// Falling-edge triggered (`.latch fe`).
    FallingEdge,
    /// Rising-edge triggered (`.latch re`).
    RisingEdge,
    /// Active-high level-sensitive (`.latch ah`).
    ActiveHigh,
    /// Active-low level-sensitive (`.latch al`).
    ActiveLow,
    /// Asynchronous latch (`.latch as`).
    Asynchronous,
}

/// The initialisation value of a flip-flop.
#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub enum FlipFlopInit {
    /// Initialised to a constant: `0` or `1`.
    Const(bool),
    /// Don't-care initial value (`2` in BLIF).
    DontCare,
    /// Unknown initial value (`3` in BLIF, or omitted).
    Unknown,
}

/// A single FSM transition in a `.start_kiss` / `.end_kiss` block.
#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct FSMTransition<'s> {
    /// Input pattern (vector of tristate values).
    pub input: SmallVec<[Tristate; 8]>,
    /// Name of the current state.
    pub current_state: &'s str,
    /// Name of the next state.
    pub next_state: &'s str,
    /// Output pattern (vector of tristate values).
    pub output: SmallVec<[Tristate; 8]>,
}

/// Parse a string of tristate characters into a collection.
fn str_to_tristates<C: FromIterator<Tristate>>(s: &str) -> Result<C, ()> {
    s.bytes()
        .map(|x| {
            let x = [x];
            let x = unsafe { str::from_utf8_unchecked(&x) };
            x.parse()
        })
        .collect::<Result<_, _>>()
}

/// The phase relationship for an input delay constraint.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DelayConstraintPhase {
    /// The output signal is inverting relative to the input.
    Inverting,
    /// The output signal is non-inverting relative to the input.
    NonInverting,
    /// The phase relationship is unknown.
    Unknown,
}

impl DelayConstraintPhase {
    fn parse(s: &str) -> Result<Self, BlifParserError> {
        match s.to_lowercase().as_str() {
            "inv" => Ok(Self::Inverting),
            "noinv" | "noninv" => Ok(Self::NonInverting),
            "unknown" => Ok(Self::Unknown),
            _ => Err(BlifParserError::Invalid),
        }
    }
}

/// An input delay constraint (from the `.delay` command with 8 arguments).
///
/// This describes the timing characteristics of a single input pin.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct InputDelayConstraint {
    /// The input signal name.
    ///
    /// `Str<0>` means the string is always heap-allocated (no inline capacity).
    pub input: Str<0>,
    /// Phase relationship (inverting, non-inverting, or unknown).
    pub phase: DelayConstraintPhase,
    /// The load capacitance presented by this input.
    pub load: f32,
    /// The maximum load capacitance.
    pub max_load: f32,
    /// Block delay for rising transitions.
    pub block_rise: f32,
    /// Drive resistance for rising transitions.
    pub drive_rise: f32,
    /// Block delay for falling transitions.
    pub block_fall: f32,
    /// Drive resistance for falling transitions.
    pub drive_fall: f32,
}

/// Whether a timing event occurs before or after a reference point.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum BeforeAfter {
    /// Before the reference event.
    Before,
    /// After the reference event.
    After,
}

/// A time value that is relative to a named clock event.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct TimeRelativeToEvent {
    /// Whether the time is before or after the event.
    pub ba: BeforeAfter,
    /// The name of the event (clock signal).
    ///
    /// `Str<0>` means the string is always heap-allocated (no inline capacity).
    pub event: Str<0>,
}

/// The arrival time of a signal, optionally relative to a clock event.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct SignalArrivalTime {
    /// The signal name.
    ///
    /// `Str<0>` means the string is always heap-allocated (no inline capacity).
    pub signal: Str<0>,
    /// Rise arrival time.
    pub rise: f32,
    /// Fall arrival time.
    pub fall: f32,
    /// If `Some`, the arrival time is relative to this clock event.
    pub event_relative: Option<TimeRelativeToEvent>,
}

/// Drive strength (rise/fall resistance) for a signal.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct SignalDrive {
    /// The signal name.
    ///
    /// `Str<0>` means the string is always heap-allocated (no inline capacity).
    pub signal: Str<0>,
    /// Rise drive resistance.
    pub rise: f32,
    /// Fall drive resistance.
    pub fall: f32,
}

/// A load value for a signal.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct SignalLoad {
    /// The signal name.
    ///
    /// `Str<0>` means the string is always heap-allocated (no inline capacity).
    pub signal: Str<0>,
    /// The load value (capacitance, time, etc. depending on context).
    pub load: f32,
}

/// Model-level delay constraints.
///
/// These correspond to the various `.delay`-family and timing constraint
/// commands in BLIF and BLIF-MV.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ModelDelayConstraint {
    /// Full input delay constraint (8 parameter `.delay` command).
    Input(InputDelayConstraint),
    /// Wire load slope for the model (`.wire_load_slope`).
    WireLoadSlope(f32),
    /// Wire loads for the model (`.wire`).
    WireLoads(Vec<f32>),
    /// Input arrival time (`.input_arrival`).
    InputArrivalTime(SignalArrivalTime),
    /// Output required time (`.output_required`).
    OutputRequiredTime(SignalArrivalTime),
    /// Default input arrival time for all inputs not explicitly specified
    /// (`.default_input_arrival`).
    DefaultInputArrivalTime((f32, f32)),
    /// Default output required time for all outputs not explicitly specified
    /// (`.default_output_required`).
    DefaultOutputRequiredTime((f32, f32)),
    /// Input drive strength (`.input_drive`).
    InputDrive(SignalDrive),
    /// Default input drive strength (`.default_input_drive`).
    DefaultInputDrive((f32, f32)),
    /// Maximum input load (`.max_input_load`).
    MaxInputLoad(SignalLoad),
    /// Default maximum input load (`.default_max_input_load`).
    DefaultMaxInputLoad(f32),
    /// Output load (`.output_load`).
    OutputLoad(SignalLoad),
    /// Default output load (`.default_output_load`).
    DefaultOutputLoad(f32),
    /// Global AND-gate delay (`.delay <float>` or `.and_gate_delay`).
    AndGateDelay(f32),
    /// Per-input required time (`.input_required`).
    InputRequired(SignalLoad),
    /// Per-output arrival time (`.output_arrival`).
    OutputArrival(SignalLoad),
    /// Per-input-to-output delay (`.delay <in-sig> <out-sig> <delay>`).
    /// ABC extension.
    DelayPerPair {
        /// Input signal name.
        in_sig: Str<0>,
        /// Output signal name.
        out_sig: Str<0>,
        /// Delay value.
        delay: f32,
    },
}

/// Tokenise a BLIF line into space-separated tokens, preserving parenthesised
/// groups as single tokens.
fn tokenize(src: &str) -> Vec<&str> {
    let mut out = vec![];
    let iter = src.chars().enumerate().peekable();

    let mut first = None;
    let mut ind = 0;

    for x in iter {
        match (x.1, first, ind) {
            ('(', _, _) => {
                if ind == 0 {
                    first = Some(x.0);
                }
                ind += 1;
            }
            (')', Some(f), _) => {
                if ind > 0 {
                    ind -= 1;
                }
                if ind == 0 {
                    out.push(&src[f..=x.0]);
                    first = None;
                }
            }
            (' ', None, 0) => {}
            (' ', Some(f), 0) => {
                out.push(&src[f..x.0]);
                first = None;
            }
            (_, None, _) => {
                first = Some(x.0);
            }
            (_, Some(_), _) => {}
        }
    }

    if let Some(s) = first {
        out.push(&src[s..]);
    }

    out
}

#[test]
fn test_tokenize_simple() {
    assert_eq!(
        tokenize("This should be   split at          spaces   "),
        vec!["This", "should", "be", "split", "at", "spaces"],
    );
    assert_eq!(
        tokenize("    This should be   split at          spaces   "),
        vec!["This", "should", "be", "split", "at", "spaces"],
    );
}

#[test]
fn test_tokenize_parens() {
    assert_eq!(
        tokenize("This (is some example) text (with (some nesting) yay) works"),
        vec![
            "This",
            "(is some example)",
            "text",
            "(with (some nesting) yay)",
            "works"
        ],
    );
}

/// Parse the contents of a single `.model` ... `.end` block.
///
/// This function drives the provided [`ModelConsumer`] by feeding it
/// per-command callbacks as each directive is encountered.
fn parse_mod(
    name: &str,
    consumer: &mut impl ModelConsumer,
    lines: &mut Peekable<impl Iterator<Item = impl AsRef<str>>>,
) -> Result<(), BlifParserError> {
    // BLIF-MV: .spec <file-name> — appears immediately after .model
    parse_padding(lines);
    if is_kw(lines, ".spec") {
        let _line = next_stmt(lines)?.unwrap();
    }

    let inputs = {
        parse_padding(lines);
        if is_kw(lines, ".inputs") || is_kw(lines, ".input") {
            let line = next_stmt(lines)?.unwrap();
            let line = line.as_ref();
            let mut args = line.split(' ');
            let _cmd = args.next().ok_or(BlifParserError::Invalid)?;

            Some(args.map(|x| x.into()).collect())
        } else {
            None
        }
    };

    let outputs = {
        parse_padding(lines);
        if is_kw(lines, ".outputs") || is_kw(lines, ".output") {
            let line = next_stmt(lines)?.unwrap();
            let line = line.as_ref();
            let mut args = line.split(' ');
            let _cmd = args.next().ok_or(BlifParserError::Invalid)?;

            Some(args.map(|x| x.into()).collect())
        } else {
            None
        }
    };

    let clocks = {
        parse_padding(lines);
        if is_kw(lines, ".clock") {
            let line = next_stmt(lines)?.unwrap();
            let line = line.as_ref();
            let mut args = line.split(' ');
            let _cmd = args.next().ok_or(BlifParserError::Invalid)?;

            args.map(|x| x.into()).collect()
        } else {
            vec![]
        }
    };

    let main_consumer = consumer;
    let mut consumer = main_consumer.model(ModelMeta {
        name: name.into(),
        inputs,
        outputs,
        clocks,
    });

    while {
        parse_padding(lines);
        lines.peek().is_some()
    } {
        let mut line = next_stmt(lines)?.unwrap();
        let extdc = if line.as_ref() == ".exdc" {
            line = next_stmt(lines)?.unwrap();
            true
        } else {
            false
        };

        let line = line.as_ref().trim();
        if line == ".end" {
            break;
        }

        let mut args = tokenize(line).into_iter();
        let cmd = args.next().ok_or(BlifParserError::Invalid)?;

        match cmd {
            ".names" => {
                let mut inputs: Vec<_> = args.map(|x| x.into()).collect();
                let output = inputs.pop().ok_or(BlifParserError::MissingArgs)?;

                let mut gate = consumer.gate(GateMeta {
                    inputs,
                    output,
                    external_dc: extdc,
                });

                while {
                    parse_padding(lines);
                    lines.peek().is_some_and(|x| !x.as_ref().starts_with("."))
                } {
                    let l = next_stmt(lines)?.unwrap();
                    let l = l.as_ref();

                    let (l, r) = if l.contains(' ') {
                        l.split_once(' ').unwrap()
                    } else {
                        ("", l)
                    };

                    let invs = str_to_tristates(l).map_err(|_| BlifParserError::Invalid)?;
                    let outvs = match r {
                        "0" => Some(false),
                        "1" => Some(true),
                        "x" | "n" => None,
                        _ => Err(BlifParserError::Invalid)?,
                    };

                    gate.entry(invs, outvs);
                }

                consumer.gate_done(gate);
            }

            ".latch" => {
                let inp = args.next().ok_or(BlifParserError::MissingArgs)?;
                let out = args.next().ok_or(BlifParserError::MissingArgs)?;
                let mut init_val = FlipFlopInit::Unknown;
                let ty = args
                    .next()
                    .map(|t| match t {
                        "fe" => Ok(Some(FlipFlopType::FallingEdge)),
                        "re" => Ok(Some(FlipFlopType::RisingEdge)),
                        "ah" => Ok(Some(FlipFlopType::ActiveHigh)),
                        "al" => Ok(Some(FlipFlopType::ActiveLow)),
                        "as" => Ok(Some(FlipFlopType::Asynchronous)),
                        "0" => {
                            init_val = FlipFlopInit::Const(false);
                            Ok(None)
                        }
                        "1" => {
                            init_val = FlipFlopInit::Const(true);
                            Ok(None)
                        }
                        "2" => {
                            init_val = FlipFlopInit::DontCare;
                            Ok(None)
                        }
                        "3" => {
                            init_val = FlipFlopInit::Unknown;
                            Ok(None)
                        }
                        _ => Err(BlifParserError::Invalid),
                    })
                    .unwrap_or(Ok(None))?;
                let mut ctrl = args.next().map(|x| x.into());
                if ctrl.as_ref().is_some_and(|x| x == "NIL") {
                    ctrl = None;
                }
                if let Some(x) = args.next() {
                    init_val = match x {
                        "0" => FlipFlopInit::Const(false),
                        "1" => FlipFlopInit::Const(true),
                        "2" => FlipFlopInit::DontCare,
                        "3" => FlipFlopInit::Unknown,
                        _ => Err(BlifParserError::Invalid)?,
                    };
                }

                // consume optional register class (ABC extension) - silently ignore
                let _ = args.next();

                consumer.ff(FlipFlop {
                    ty,
                    input: inp.into(),
                    output: out.into(),
                    clock: ctrl,
                    init: init_val,
                });
            }

            ".gate" => {
                let name = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                let maps = args
                    .map(|x| {
                        x.split_once('=')
                            .ok_or(BlifParserError::Invalid)
                            .map(|(k, v)| (k.into(), v.into()))
                    })
                    .collect::<Result<_, _>>()?;

                consumer.lib_gate(LibGate { name, maps });
            }

            ".mlatch" => {
                let name = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                let mut args = args.peekable();
                let mut maps = vec![];
                while args.peek().is_some_and(|x| x.contains("=")) {
                    let (k, v) = args
                        .next()
                        .unwrap()
                        .split_once("=")
                        .ok_or(BlifParserError::Invalid)?;
                    maps.push((k.into(), v.into()));
                }
                let mut control = args.next().map(|x| x.into());
                if control.as_ref().is_some_and(|x| x == "NIL") {
                    control = None;
                }
                let init_val = if let Some(x) = args.next() {
                    Some(match x {
                        "0" => FlipFlopInit::Const(false),
                        "1" => FlipFlopInit::Const(true),
                        "2" => FlipFlopInit::DontCare,
                        "3" => FlipFlopInit::Unknown,
                        _ => Err(BlifParserError::Invalid)?,
                    })
                } else {
                    None
                };

                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }

                consumer.lib_ff(LibFlipFlop {
                    name,
                    maps,
                    clock: control,
                    init: init_val.unwrap_or(FlipFlopInit::Unknown),
                });
            }

            ".subckt" => {
                let raw = args.next().ok_or(BlifParserError::MissingArgs)?;
                let (name, instance_name) = if let Some((m, inst)) = raw.split_once('|') {
                    (m, Some(inst))
                } else {
                    (raw, None)
                };
                let maps = args
                    .map(|x| {
                        x.split_once('=')
                            .ok_or(BlifParserError::Invalid)
                            .map(|(k, v)| (k.into(), v.into()))
                    })
                    .collect::<Result<_, _>>()?;

                consumer.sub_model(name, maps, instance_name);
            }

            ".search" => {
                let path = args.next().ok_or(BlifParserError::MissingArgs)?;

                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }

                main_consumer.search(path);
            }

            ".start_kiss" => {
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }

                let num_ins: usize = {
                    parse_padding(lines);
                    let line = next_stmt(lines)?.unwrap();
                    let line = line.as_ref().trim();
                    let mut args = line.split(' ');
                    let cmd = args.next().ok_or(BlifParserError::Invalid)?;

                    if cmd != ".i" {
                        Err(BlifParserError::UnknownKw(cmd.to_string()))?
                    }

                    let v = args
                        .next()
                        .ok_or(BlifParserError::MissingArgs)?
                        .parse()
                        .map_err(|_| BlifParserError::Invalid)?;

                    if args.next().is_some() {
                        Err(BlifParserError::TooManyArgs)?
                    }

                    v
                };

                let num_outs: usize = {
                    parse_padding(lines);
                    let line = next_stmt(lines)?.unwrap();
                    let line = line.as_ref().trim();
                    let mut args = line.split(' ');
                    let cmd = args.next().ok_or(BlifParserError::Invalid)?;

                    if cmd != ".o" {
                        Err(BlifParserError::UnknownKw(cmd.to_string()))?
                    }

                    let v = args
                        .next()
                        .ok_or(BlifParserError::MissingArgs)?
                        .parse()
                        .map_err(|_| BlifParserError::Invalid)?;

                    if args.next().is_some() {
                        Err(BlifParserError::TooManyArgs)?
                    }

                    v
                };

                let _num_terms: Option<usize> = {
                    parse_padding(lines);
                    if is_kw(lines, ".p") {
                        let line = next_stmt(lines)?.unwrap();
                        let line = line.as_ref().trim();

                        let mut args = line.split(' ');
                        let _cmd = args.next().ok_or(BlifParserError::Invalid)?;

                        let v = args
                            .next()
                            .ok_or(BlifParserError::MissingArgs)?
                            .parse()
                            .map_err(|_| BlifParserError::Invalid)?;

                        if args.next().is_some() {
                            Err(BlifParserError::TooManyArgs)?
                        }

                        Some(v)
                    } else {
                        None
                    }
                };

                let _num_states: Option<usize> = {
                    parse_padding(lines);
                    if is_kw(lines, ".s") {
                        let line = next_stmt(lines)?.unwrap();
                        let line = line.as_ref().trim();

                        let mut args = line.split(' ');
                        let _cmd = args.next().ok_or(BlifParserError::Invalid)?;

                        let v = args
                            .next()
                            .ok_or(BlifParserError::MissingArgs)?
                            .parse()
                            .map_err(|_| BlifParserError::Invalid)?;

                        if args.next().is_some() {
                            Err(BlifParserError::TooManyArgs)?
                        }

                        Some(v)
                    } else {
                        None
                    }
                };

                let reset_state: Option<String> = {
                    parse_padding(lines);
                    if is_kw(lines, ".r") {
                        let line = next_stmt(lines)?.unwrap();
                        let line = line.as_ref().trim();

                        let mut args = line.split(' ');
                        let _cmd = args.next().ok_or(BlifParserError::Invalid)?;

                        let v = args.next().ok_or(BlifParserError::MissingArgs)?;

                        if args.next().is_some() {
                            Err(BlifParserError::TooManyArgs)?
                        }

                        Some(v.to_string())
                    } else {
                        None
                    }
                };

                let mut fsm = consumer.fsm(num_ins, num_outs, reset_state.as_deref());

                while {
                    parse_padding(lines);
                    lines.peek().is_some_and(|x| x.as_ref() != ".end_kiss")
                } {
                    let line = next_stmt(lines)?.unwrap();
                    let line = line.as_ref().trim();

                    let mut args = line.split(' ');

                    let input = str_to_tristates(args.next().ok_or(BlifParserError::Invalid)?)
                        .map_err(|_| BlifParserError::Invalid)?;
                    let current_state = args.next().ok_or(BlifParserError::Invalid)?.to_string();
                    let next_state = args.next().ok_or(BlifParserError::Invalid)?.to_string();
                    let output = str_to_tristates(args.next().ok_or(BlifParserError::Invalid)?)
                        .map_err(|_| BlifParserError::Invalid)?;

                    fsm.add_transition(FSMTransition {
                        input,
                        current_state: current_state.as_str(),
                        next_state: next_state.as_str(),
                        output,
                    });
                }

                {
                    parse_padding(lines);
                    let line = next_stmt(lines)?.ok_or(BlifParserError::UnexpectedEnd)?;
                    let line = line.as_ref().trim();
                    if line != ".end_kiss" {
                        Err(BlifParserError::Invalid)?
                    }
                };

                let latch_order: Option<Vec<String>> = {
                    parse_padding(lines);
                    if is_kw(lines, ".latch_order") {
                        let line = next_stmt(lines)?.unwrap();
                        let line = line.as_ref().trim();

                        let mut args = line.split(' ');
                        let _cmd = args.next().ok_or(BlifParserError::Invalid)?;

                        Some(args.map(|x| x.to_string()).collect())
                    } else {
                        None
                    }
                };

                let mut code_mapping = vec![];

                while {
                    parse_padding(lines);
                    is_kw(lines, ".code")
                } {
                    let line = next_stmt(lines)?.unwrap();
                    let line = line.as_ref().trim();
                    let mut args = line.split(' ');
                    let _cmd = args.next().ok_or(BlifParserError::Invalid)?;

                    let state = args.next().ok_or(BlifParserError::Invalid)?;
                    let value = args
                        .next()
                        .ok_or(BlifParserError::Invalid)?
                        .chars()
                        .map(|x| match x {
                            '0' => Ok(false),
                            '1' => Ok(true),
                            _ => Err(BlifParserError::Invalid),
                        })
                        .collect::<Result<_, _>>()?;

                    code_mapping.push((state.to_string(), value));
                }

                consumer.fsm_done(
                    fsm,
                    latch_order,
                    if code_mapping.is_empty() {
                        None
                    } else {
                        Some(code_mapping)
                    },
                );
            }

            ".cname" => {
                let arg = args.next().ok_or(BlifParserError::MissingArgs)?;

                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }

                consumer.attr(CellAttr::CellName(arg));
            }

            ".attr" => {
                let key = args.next().ok_or(BlifParserError::MissingArgs)?;

                let val = args.fold(String::new(), |acc, x| {
                    let mut acc = acc;
                    acc.push_str(x);
                    acc
                });

                consumer.attr(CellAttr::Attr {
                    key,
                    val: val.as_str(),
                });
            }

            ".param" => {
                let key = args.next().ok_or(BlifParserError::MissingArgs)?;

                let val = args.fold(String::new(), |acc, x| {
                    let mut acc = acc;
                    acc.push_str(x);
                    acc
                });

                consumer.attr(CellAttr::Param {
                    key,
                    val: val.as_str(),
                });
            }

            ".barbuff" | ".barbuf" | ".conn" => {
                let from = args.next().ok_or(BlifParserError::MissingArgs)?;
                let to = args.next().ok_or(BlifParserError::MissingArgs)?;

                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }

                consumer.connect(from, to);
            }

            ".area" => {
                let val = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;

                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }

                consumer.set_area(val);
            }

            ".cycle" => {
                let val = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;

                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }

                consumer.set_cycle_time(val);
            }

            ".clock_event" => {
                let percent = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;

                fn parse_rfcn(src: &str) -> Result<(ClockEdgeKind, &str), BlifParserError> {
                    let (edge, name) = src.split_once('\'').ok_or(BlifParserError::Invalid)?;
                    Ok((
                        match edge {
                            "r" => ClockEdgeKind::Rise,
                            "f" => ClockEdgeKind::Fall,
                            _ => Err(BlifParserError::Invalid)?,
                        },
                        name,
                    ))
                }

                let events = args
                    .map(|x| -> Result<_, _> {
                        Ok(if x.chars().next().is_some_and(|x| x == '(') {
                            if !x.ends_with(')') {
                                Err(BlifParserError::Invalid)?;
                            }
                            let tokens = tokenize(&x[1..x.len() - 1]);
                            if tokens.len() != 3 {
                                Err(BlifParserError::Invalid)?
                            }
                            let (edge, name) = parse_rfcn(tokens[0])?;
                            ClockEvent {
                                edge,
                                clock_name: name.into(),
                                before_after: Some((
                                    tokens[1].parse().map_err(|_| BlifParserError::Invalid)?,
                                    tokens[2].parse().map_err(|_| BlifParserError::Invalid)?,
                                )),
                            }
                        } else {
                            let (edge, name) = parse_rfcn(x)?;
                            ClockEvent {
                                edge,
                                clock_name: name.into(),
                                before_after: None,
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                consumer.clock_events(ClockEvents { percent, events });
            }

            ".delay" => {
                let arg0 = args.next().ok_or(BlifParserError::MissingArgs)?;
                let arg1 = args.next();

                match arg1 {
                    None => {
                        // .delay <delay>  (global)
                        let delay = arg0.parse().map_err(|_| BlifParserError::Invalid)?;
                        consumer.model_delay_constraint(ModelDelayConstraint::AndGateDelay(delay));
                    }
                    Some(arg1_val)
                        if arg1_val.parse::<f32>().is_ok() && args.clone().next().is_none() =>
                    {
                        // .delay <signal> <delay>  (per-signal) — second token is a float and no more args
                        let signal: Str<0> = arg0.into();
                        let time = arg1_val.parse().map_err(|_| BlifParserError::Invalid)?;
                        consumer.model_delay_constraint(ModelDelayConstraint::InputRequired(
                            SignalLoad { signal, load: time },
                        ));
                    }
                    Some(arg1_val) => {
                        let arg2 = args.next();
                        if let Some(arg2_val) = arg2 {
                            if arg2_val.parse::<f32>().is_ok() && args.clone().next().is_none() {
                                // .delay <in-sig> <out-sig> <delay>  (per-pair) — ABC extension
                                let in_sig: Str<0> = arg0.into();
                                let out_sig: Str<0> = arg1_val.into();
                                let delay =
                                    arg2_val.parse().map_err(|_| BlifParserError::Invalid)?;
                                consumer.model_delay_constraint(
                                    ModelDelayConstraint::DelayPerPair {
                                        in_sig,
                                        out_sig,
                                        delay,
                                    },
                                );
                            } else {
                                // .delay <in-name> <phase> <load> <max-load> <brise> <drise> <bfall> <dfall>  (original BLIF)
                                let input: Str<0> = arg0.into();
                                let phase = DelayConstraintPhase::parse(arg1_val)?;
                                let load =
                                    arg2_val.parse().map_err(|_| BlifParserError::Invalid)?;
                                let max_load = args
                                    .next()
                                    .ok_or(BlifParserError::MissingArgs)?
                                    .parse()
                                    .map_err(|_| BlifParserError::Invalid)?;
                                let block_rise = args
                                    .next()
                                    .ok_or(BlifParserError::MissingArgs)?
                                    .parse()
                                    .map_err(|_| BlifParserError::Invalid)?;
                                let drive_rise = args
                                    .next()
                                    .ok_or(BlifParserError::MissingArgs)?
                                    .parse()
                                    .map_err(|_| BlifParserError::Invalid)?;
                                let block_fall = args
                                    .next()
                                    .ok_or(BlifParserError::MissingArgs)?
                                    .parse()
                                    .map_err(|_| BlifParserError::Invalid)?;
                                let drive_fall = args
                                    .next()
                                    .ok_or(BlifParserError::MissingArgs)?
                                    .parse()
                                    .map_err(|_| BlifParserError::Invalid)?;

                                if args.next().is_some() {
                                    Err(BlifParserError::TooManyArgs)?
                                }

                                consumer.model_delay_constraint(ModelDelayConstraint::Input(
                                    InputDelayConstraint {
                                        input,
                                        phase,
                                        load,
                                        max_load,
                                        block_rise,
                                        drive_rise,
                                        block_fall,
                                        drive_fall,
                                    },
                                ));
                            }
                        }
                    }
                }
            }

            ".wire_load_slope" => {
                let load = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }

                consumer.model_delay_constraint(ModelDelayConstraint::WireLoadSlope(load));
            }

            ".wire" => {
                let loads = args
                    .map(|x| x.parse())
                    .collect::<Result<_, _>>()
                    .map_err(|_| BlifParserError::Invalid)?;
                consumer.model_delay_constraint(ModelDelayConstraint::WireLoads(loads));
            }

            ".input_arrival" => {
                let signal = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                let rise = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                let fall = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                let event_relative = if let Some(ba) = args.next() {
                    let ba = match ba {
                        "b" => BeforeAfter::Before,
                        "a" => BeforeAfter::After,
                        _ => Err(BlifParserError::Invalid)?,
                    };
                    let event = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                    Some(TimeRelativeToEvent { ba, event })
                } else {
                    None
                };
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::InputArrivalTime(
                    SignalArrivalTime {
                        signal,
                        rise,
                        fall,
                        event_relative,
                    },
                ));
            }

            ".output_required" => {
                let signal = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                let rise = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                let fall = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                let event_relative = if let Some(ba) = args.next() {
                    let ba = match ba {
                        "b" => BeforeAfter::Before,
                        "a" => BeforeAfter::After,
                        _ => Err(BlifParserError::Invalid)?,
                    };
                    let event = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                    Some(TimeRelativeToEvent { ba, event })
                } else {
                    None
                };
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::OutputRequiredTime(
                    SignalArrivalTime {
                        signal,
                        rise,
                        fall,
                        event_relative,
                    },
                ));
            }

            ".default_input_arrival" => {
                let rise = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                let fall = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::DefaultInputArrivalTime((
                    rise, fall,
                )));
            }

            ".default_output_required" => {
                let rise = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                let fall = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::DefaultOutputRequiredTime((
                    rise, fall,
                )));
            }

            ".input_drive" => {
                let signal = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                let rise = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                let fall = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::InputDrive(SignalDrive {
                    signal,
                    rise,
                    fall,
                }));
            }

            ".default_input_drive" => {
                let rise = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                let fall = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer
                    .model_delay_constraint(ModelDelayConstraint::DefaultInputDrive((rise, fall)));
            }

            ".output_load" => {
                let signal = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                let load = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::OutputLoad(SignalLoad {
                    signal,
                    load,
                }));
            }

            ".default_output_load" => {
                let load = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::DefaultOutputLoad(load));
            }

            ".max_input_load" => {
                let signal = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                let load = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::MaxInputLoad(SignalLoad {
                    signal,
                    load,
                }));
            }

            ".default_max_input_load" => {
                let load = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::DefaultMaxInputLoad(load));
            }

            ".and_gate_delay" => {
                let delay = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::AndGateDelay(delay));
            }

            ".input_required" => {
                let signal = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                let time = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::InputRequired(SignalLoad {
                    signal,
                    load: time,
                }));
            }

            ".output_arrival" => {
                let signal = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                let time = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse()
                    .map_err(|_| BlifParserError::Invalid)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.model_delay_constraint(ModelDelayConstraint::OutputArrival(SignalLoad {
                    signal,
                    load: time,
                }));
            }

            ".attrib" | ".no_merge" => {
                // Box attributes and no-merge directives are parsed but currently ignored.
                let _ = args.collect::<Vec<_>>();
            }

            ".blackbox" => {
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
            }

            // BLIF-MV: .short <in> <out> — buffer (equivalent to .conn)
            ".short" => {
                let from = args.next().ok_or(BlifParserError::MissingArgs)?;
                let to = args.next().ok_or(BlifParserError::MissingArgs)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.connect(from, to);
            }

            // BLIF-MV: .constraint <signal> ...
            ".constraint" => {
                let signals: Vec<_> = args.map(|x| x.into()).collect();
                consumer.constraint(&signals);
            }

            // BLIF-MV: .onehot <signal> ...
            ".onehot" => {
                let signals: Vec<_> = args.map(|x| x.into()).collect();
                consumer.onehot(&signals);
            }

            // BLIF-MV: .reset <signal> \n <value>
            ".reset" => {
                let signal = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                parse_padding(lines);
                let line = next_stmt(lines)?.ok_or(BlifParserError::UnexpectedEnd)?;
                let value =
                    str_to_tristates(line.as_ref().trim()).map_err(|_| BlifParserError::Invalid)?;
                consumer.reset(signal, value);
            }

            // BLIF-MV: .ltlformula "<LTL string>"
            ".ltlformula" => {
                // the formula may contain spaces, so use the rest of the line
                let formula = args.fold(String::new(), |acc, x| {
                    let mut acc = acc;
                    if !acc.is_empty() {
                        acc.push(' ');
                    }
                    acc.push_str(x);
                    acc
                });
                consumer.ltlformula(&formula);
            }

            // BLIF-MV: .spec <file-name>
            ".spec" => {
                let filename = args.next().ok_or(BlifParserError::MissingArgs)?;
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.spec(filename);
            }

            // BLIF-MV / Yosys: .gateinit <signal>=<init-val>
            ".gateinit" => {
                let arg = args.next().ok_or(BlifParserError::MissingArgs)?;
                let (signal, val) = arg.split_once('=').ok_or(BlifParserError::Invalid)?;
                let value = match val {
                    "0" => FlipFlopInit::Const(false),
                    "1" => FlipFlopInit::Const(true),
                    "2" => FlipFlopInit::DontCare,
                    "3" => FlipFlopInit::Unknown,
                    _ => Err(BlifParserError::Invalid)?,
                };
                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }
                consumer.gateinit(signal.into(), value);
            }

            // BLIF-MV: .mv <var> ... <nvalues> [<val-name> ...]
            ".mv" => {
                // re-tokenize the line for simpler parsing
                let all_args: Vec<_> = tokenize(line).into_iter().collect();
                // skip ".mv"
                let iter = all_args.into_iter().skip(1);
                let mut variables = vec![];
                let mut nvalues: Option<usize> = None;
                let mut value_names = vec![];

                // collect variables (non-numeric tokens), then nvalues (first numeric), then value names
                for a in iter {
                    if nvalues.is_some() {
                        value_names.push(a.to_string());
                    } else if let Ok(n) = a.parse::<usize>() {
                        nvalues = Some(n);
                    } else {
                        variables.push(a.into());
                    }
                }

                let nvalues = nvalues.ok_or(BlifParserError::MissingArgs)?;
                consumer.mv(variables, nvalues, value_names);
            }

            // ABC Extended BLIF: .flop D=<in> Q=<out> C=<clk> [S=<set>] [R=<reset>] [E=<enable>] [async] [negedge] [init=<val>]
            ".flop" => {
                let mut input: Option<&str> = None;
                let mut output: Option<&str> = None;
                let mut clock: Option<Str<16>> = None;
                let mut init_val = FlipFlopInit::Unknown;
                let mut ty: Option<FlipFlopType> = None;

                for arg in args {
                    if let Some(val) = arg.strip_prefix("D=") {
                        input = Some(val);
                    } else if let Some(val) = arg.strip_prefix("Q=") {
                        output = Some(val);
                    } else if let Some(val) = arg.strip_prefix("C=") {
                        clock = Some(val.into());
                    } else if arg == "async" {
                        ty = Some(FlipFlopType::Asynchronous);
                    } else if arg == "negedge" {
                        ty = Some(FlipFlopType::FallingEdge);
                    } else if let Some(val) = arg.strip_prefix("init=") {
                        init_val = match val {
                            "0" => FlipFlopInit::Const(false),
                            "1" => FlipFlopInit::Const(true),
                            "2" => FlipFlopInit::DontCare,
                            "3" => FlipFlopInit::Unknown,
                            _ => Err(BlifParserError::Invalid)?,
                        };
                    } else if let Some(_val) = arg.strip_prefix("S=") {
                        // set pin — not currently represented in FlipFlop, silently ignore
                    } else if let Some(_val) = arg.strip_prefix("R=") {
                        // reset pin — not currently represented in FlipFlop, silently ignore
                    } else if let Some(_val) = arg.strip_prefix("E=") {
                        // enable pin — not currently represented in FlipFlop, silently ignore
                    }
                }

                let input = input.ok_or(BlifParserError::MissingArgs)?;
                let output = output.ok_or(BlifParserError::MissingArgs)?;
                // negedge implies FallingEdge if no other type specified
                // if ty is None and negedge was specified, it's already set to FallingEdge

                consumer.ff(FlipFlop {
                    ty,
                    input: input.into(),
                    output: output.into(),
                    clock,
                    init: init_val,
                });
            }

            // ABC alias: .subcircuit (same as .subckt)
            ".subcircuit" => {
                let raw = args.next().ok_or(BlifParserError::MissingArgs)?;
                let (name, instance_name) = if let Some((m, inst)) = raw.split_once('|') {
                    (m, Some(inst))
                } else {
                    (raw, None)
                };
                let maps = args
                    .map(|x| {
                        x.split_once('=')
                            .ok_or(BlifParserError::Invalid)
                            .map(|(k, v)| (k.into(), v.into()))
                    })
                    .collect::<Result<_, _>>()?;
                consumer.sub_model(name, maps, instance_name);
            }

            // BLIF-MV: .table <in1> <in2> ... -> <out1> <out2> ...
            // handles the multi-valued table, similar to .names but with -> separator
            ".table" => {
                // collect args until '->', then outputs after
                let mut inputs: Vec<Str<16>> = vec![];
                let mut outputs: Vec<Str<16>> = vec![];
                let mut seen_arrow = false;
                for arg in args {
                    if arg == "->" {
                        seen_arrow = true;
                    } else if !seen_arrow {
                        inputs.push(arg.into());
                    } else {
                        outputs.push(arg.into());
                    }
                }

                if !seen_arrow || outputs.is_empty() {
                    Err(BlifParserError::Invalid)?
                }

                // For now, handle single-output tables like .names
                if outputs.len() == 1 {
                    let output = outputs.pop().unwrap();
                    let mut gate = consumer.gate(GateMeta {
                        inputs,
                        output,
                        external_dc: extdc,
                    });

                    while {
                        parse_padding(lines);
                        lines.peek().is_some_and(|x| !x.as_ref().starts_with("."))
                    } {
                        let l = next_stmt(lines)?.unwrap();
                        let l = l.as_ref();

                        let (l, r) = if l.contains(' ') {
                            l.split_once(' ').unwrap()
                        } else {
                            ("", l)
                        };

                        let invs = str_to_tristates(l).map_err(|_| BlifParserError::Invalid)?;
                        let outvs = match r {
                            "0" => Some(false),
                            "1" => Some(true),
                            "x" | "n" => None,
                            _ => Err(BlifParserError::Invalid)?,
                        };

                        gate.entry(invs, outvs);
                    }

                    consumer.gate_done(gate);
                } else {
                    // multi-output table — not fully supported, skip table lines
                    while {
                        parse_padding(lines);
                        lines.peek().is_some_and(|x| !x.as_ref().starts_with("."))
                    } {
                        let _l = next_stmt(lines)?.unwrap();
                    }
                }
            }

            // SIS: .cover <nin> <nout> <nterms> — alternative to .names
            ".cover" => {
                let _nin = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse::<usize>()
                    .map_err(|_| BlifParserError::Invalid)?;
                let nout = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse::<usize>()
                    .map_err(|_| BlifParserError::Invalid)?;
                if nout != 1 {
                    Err(BlifParserError::Invalid)?
                }
                let _nterms = args
                    .next()
                    .ok_or(BlifParserError::MissingArgs)?
                    .parse::<usize>()
                    .map_err(|_| BlifParserError::Invalid)?;

                // next line: <input-list> <output>
                parse_padding(lines);
                let header_line = next_stmt(lines)?.ok_or(BlifParserError::UnexpectedEnd)?;
                let mut header_args = tokenize(header_line.as_ref()).into_iter();
                let output = header_args
                    .next_back()
                    .ok_or(BlifParserError::MissingArgs)?
                    .into();
                let inputs: Vec<_> = header_args.map(|x| x.into()).collect();

                let mut gate = consumer.gate(GateMeta {
                    inputs,
                    output,
                    external_dc: extdc,
                });

                while {
                    parse_padding(lines);
                    lines.peek().is_some_and(|x| !x.as_ref().starts_with("."))
                } {
                    let l = next_stmt(lines)?.unwrap();
                    let l = l.as_ref();

                    let (l, r) = if l.contains(' ') {
                        l.split_once(' ').unwrap()
                    } else {
                        ("", l)
                    };

                    let invs = str_to_tristates(l).map_err(|_| BlifParserError::Invalid)?;
                    let outvs = match r {
                        "0" => Some(false),
                        "1" => Some(true),
                        "x" | "n" => None,
                        _ => Err(BlifParserError::Invalid)?,
                    };

                    gate.entry(invs, outvs);
                }

                consumer.gate_done(gate);
            }

            _ => Err(BlifParserError::UnknownKw(cmd.to_string()))?,
        };
    }

    main_consumer.model_done(consumer);

    Ok(())
}

/// Parse a BLIF file, driving a [`ModelConsumer`] with callbacks.
///
/// # Arguments
///
/// * `file_name` - When the model name is not declared explicitly in the
///   BLIF file (i.e. no `.model <name>` line), this is used as the model
///   name. This behaviour is compliant with the BLIF specification.
///
/// * `consumer` - The consumer that will receive model/command callbacks.
///
/// * `lines` - An iterator over the lines of the BLIF file.
///
/// # Errors
///
/// Returns [`BlifParserError`] if the input is malformed.
pub fn parse_blif(
    file_name: &str,
    consumer: &mut impl ModelConsumer,
    lines: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(), BlifParserError> {
    let mut lines = lines.into_iter().peekable();
    let mut first = true;

    while {
        parse_padding(&mut lines);
        lines.peek().is_some()
    } {
        if is_kw(&mut lines, ".model") || !first {
            let line = next_stmt(&mut lines)?.unwrap();
            let line = line.as_ref();
            let mut args = line.split(' ');
            let cmd = args.next().ok_or(BlifParserError::Invalid)?;

            match cmd {
                ".search" => {
                    let path = args.next().ok_or(BlifParserError::MissingArgs)?;

                    if args.next().is_some() {
                        Err(BlifParserError::TooManyArgs)?
                    }

                    consumer.search(path);
                }

                ".model" => {
                    let mod_name = args.next().unwrap_or(file_name);
                    if args.next().is_some() {
                        Err(BlifParserError::TooManyArgs)?;
                    }
                    parse_mod(mod_name, consumer, &mut lines)?;
                }
                _ => Err(BlifParserError::UnknownKw(cmd.to_string()))?,
            }
        } else {
            parse_mod(file_name, consumer, &mut lines)?;
        }

        first = false;
    }

    Ok(())
}

#[cfg(test)]
mod tests;
