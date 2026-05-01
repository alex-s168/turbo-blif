use std::fmt;

use crate::ast::{Blif, BlifEntry, CellAttrAst, FSM, Gate, Model, ModelCmd, ModelCmdKind};
use crate::{
    BeforeAfter, ClockEdgeKind, ClockEvents, DelayConstraintPhase, FlipFlop, FlipFlopInit,
    FlipFlopType, GateMeta, LibFlipFlop, LibGate, ModelDelayConstraint, Str, Tristate,
};

/// Flavor of BLIF to emit.
///
/// Controls which syntax variants and directives are written out.
/// Constructs not supported by the chosen flavor are emitted as
/// `# <unsupported>` comments so that information is preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlifFlavor {
    /// Core BLIF (Berkeley, 1992).
    ///
    /// Emits: `.model`, `.inputs`, `.outputs`, `.clock`, `.names`, `.latch`,
    /// `.gate`, `.mlatch`, `.subckt`, `.start_kiss`/`.end_kiss`, `.exdc`,
    /// `.area`, `.cycle`, `.clock_event`, `.delay` (full input or global),
    /// `.wire_load_slope`, `.wire`, `.input_arrival`, `.output_required`,
    /// `.default_input_arrival`, `.default_output_required`, `.input_drive`,
    /// `.default_input_drive`, `.output_load`, `.default_output_load`,
    /// `.max_input_load`, `.default_max_input_load`.
    Standard,
    /// ABC extensions.
    ///
    /// Adds: `.blackbox`, `.flop`, `.and_gate_delay`, `.attrib`, `.no_merge`,
    /// `.input_required`, `.output_arrival`, per-pair `.delay`, `.subcircuit`.
    ABC,
    /// Yosys / EBLIF / VTR extensions.
    ///
    /// Adds: `.cname`, `.attr`, `.param`, `.conn`/`.barbuff`/`.barbuf`,
    /// `.gateinit`.
    Yosys,
    /// SIS extensions.
    ///
    /// Like Standard, but may emit `.cover` as an alternative to `.names`.
    Sis,
    /// BLIF-MV (SIS-MV) extensions.
    ///
    /// Adds: `.mv`, `.table`, `.short`, `.constraint`, `.onehot`, `.reset`,
    /// `.ltlformula`, `.spec`, `.gateinit`.
    SisMV,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Write the full `Blif` AST to `w` using the given `flavor`.
///
/// # Errors
///
/// Returns [`fmt::Error`] if the underlying writer fails.
pub fn write_blif<W: fmt::Write>(blif: &Blif, w: &mut W, flavor: BlifFlavor) -> fmt::Result {
    for entry in &blif.entries {
        match entry {
            BlifEntry::Model(model) => write_model(model, w, flavor)?,
        }
    }
    Ok(())
}

/// Convenience wrapper: write a `Blif` AST into a `String`.
pub fn blif_to_string(blif: &Blif, flavor: BlifFlavor) -> String {
    let mut out = String::new();
    write_blif(blif, &mut out, flavor).expect("writing to a String never fails");
    out
}

// ---------------------------------------------------------------------------
// Model-level
// ---------------------------------------------------------------------------

fn write_model<W: fmt::Write>(model: &Model, w: &mut W, flavor: BlifFlavor) -> fmt::Result {
    writeln!(w, ".model {}", model.meta.name)?;

    // .inputs
    if let Some(inputs) = &model.meta.inputs
        && !inputs.is_empty()
    {
        write!(w, ".inputs")?;
        for input in inputs {
            write!(w, " {input}")?;
        }
        writeln!(w)?;
    }

    // .outputs
    if let Some(outputs) = &model.meta.outputs
        && !outputs.is_empty()
    {
        write!(w, ".outputs")?;
        for output in outputs {
            write!(w, " {output}")?;
        }
        writeln!(w)?;
    }

    // .clock
    if !model.meta.clocks.is_empty() {
        write!(w, ".clock")?;
        for clock in &model.meta.clocks {
            write!(w, " {clock}")?;
        }
        writeln!(w)?;
    }

    // .area
    if let Some(area) = model.attr.area {
        writeln!(w, ".area {area}")?;
    }

    // Commands
    for cmd in &model.commands {
        write_model_cmd(cmd, w, flavor)?;
    }

    writeln!(w, ".end")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Per-command dispatch
// ---------------------------------------------------------------------------

fn write_model_cmd<W: fmt::Write>(cmd: &ModelCmd, w: &mut W, flavor: BlifFlavor) -> fmt::Result {
    match &cmd.kind {
        ModelCmdKind::Gate(gate) => write_gate(gate, cmd, w, flavor),
        ModelCmdKind::FF(ff) => write_ff(ff, cmd, w, flavor),
        ModelCmdKind::LibGate(lg) => write_lib_gate(lg, cmd, w, flavor),
        ModelCmdKind::LibFF(lf) => write_lib_ff(lf, cmd, w, flavor),
        ModelCmdKind::FSM(fsm) => write_fsm(fsm, cmd, w, flavor),
        ModelCmdKind::SubModel {
            name,
            map,
            instance_name,
        } => write_submodel(name, map, instance_name.as_deref(), cmd, w, flavor),
        ModelCmdKind::Connect { from, to } => write_connect(from, to, w, flavor),
        ModelCmdKind::CycleTime(t) => writeln!(w, ".cycle {t}"),
        ModelCmdKind::ClockEvents(ev) => write_clock_events(ev, w, flavor),
        ModelCmdKind::DelayConstraint(dc) => write_delay_constraint(dc, w, flavor),
        ModelCmdKind::Constraint(signals) => write_constraint(signals, w, flavor),
        ModelCmdKind::OneHot(signals) => write_onehot(signals, w, flavor),
        ModelCmdKind::Reset { signal, value } => write_reset(signal, value, w, flavor),
        ModelCmdKind::LtlFormula(formula) => write_ltlformula(formula, w, flavor),
        ModelCmdKind::Spec(filename) => write_spec(filename, w, flavor),
        ModelCmdKind::GateInit { signal, value } => write_gateinit(signal, value, w, flavor),
        ModelCmdKind::Mv {
            variables,
            nvalues,
            value_names,
        } => write_mv(variables, *nvalues, value_names, w, flavor),
    }
}

// ---------------------------------------------------------------------------
// Helper: emit trailing attributes for Yosys flavor
// ---------------------------------------------------------------------------

fn write_attrs<W: fmt::Write>(cmd: &ModelCmd, w: &mut W, flavor: BlifFlavor) -> fmt::Result {
    if matches!(flavor, BlifFlavor::Yosys) {
        for attr in &cmd.attrs {
            match attr {
                CellAttrAst::CellName(n) => {
                    writeln!(w, ".cname {n}")?;
                }
                CellAttrAst::Attr { key, val } => {
                    writeln!(w, ".attr {key} {val}")?;
                }
                CellAttrAst::Param { key, val } => {
                    writeln!(w, ".param {key} {val}")?;
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 1.2  Logic Gate (.names / .cover / .table)
// ---------------------------------------------------------------------------

fn write_gate<W: fmt::Write>(
    gate: &Gate,
    cmd: &ModelCmd,
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    if gate.meta.external_dc {
        writeln!(w, ".exdc")?;
    }

    // Decide between .names, .cover or .table based on flavor.
    //
    // We always use .names here because that's what the AST stores.
    // Alternative syntaxes (.cover, .table) are only consumed during
    // parsing and normalised into the same Gate representation.
    writeln!(w, ".names {}", format_gate_header(&gate.meta))?;

    if !gate.lut.0.is_empty() {
        write!(w, "{}", gate.lut)?;
    }

    write_attrs(cmd, w, flavor)
}

fn format_gate_header(meta: &GateMeta) -> String {
    let mut s = String::new();
    for inp in &meta.inputs {
        s.push_str(inp.as_str());
        s.push(' ');
    }
    s.push_str(meta.output.as_str());
    s
}

// ---------------------------------------------------------------------------
// 1.4  Generic Latch (.latch)  /  2.5  Extended Flip-Flop (.flop)
// ---------------------------------------------------------------------------

fn write_ff<W: fmt::Write>(
    ff: &FlipFlop,
    cmd: &ModelCmd,
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    match flavor {
        BlifFlavor::ABC => {
            // ABC can write .flop for clocked flip-flops with edges
            // and .latch for everything else.
            if ff.clock.is_some() && ff.ty.is_some() {
                write_flop(ff, cmd, w)?;
            } else {
                write_latch(ff, cmd, w)?;
            }
        }
        _ => write_latch(ff, cmd, w)?,
    }

    write_attrs(cmd, w, flavor)
}

fn write_latch<W: fmt::Write>(ff: &FlipFlop, _cmd: &ModelCmd, w: &mut W) -> fmt::Result {
    write!(w, ".latch {} {}", ff.input, ff.output)?;

    // type / init — the first positional value after in/out is ambiguous in BLIF:
    // it can be a type keyword or an init value. Write type if present.
    if let Some(ty) = &ff.ty {
        let kw = match ty {
            FlipFlopType::FallingEdge => "fe",
            FlipFlopType::RisingEdge => "re",
            FlipFlopType::ActiveHigh => "ah",
            FlipFlopType::ActiveLow => "al",
            FlipFlopType::Asynchronous => "as",
        };
        write!(w, " {kw}")?;
    }

    // clock
    if let Some(clock) = &ff.clock {
        write!(w, " {clock}")?;
    } else if ff.ty.is_some() {
        // If we wrote a type keyword but there's no clock, write NIL
        write!(w, " NIL")?;
    }

    // init
    match &ff.init {
        FlipFlopInit::Const(true) => write!(w, " 1")?,
        FlipFlopInit::Const(false) => write!(w, " 0")?,
        FlipFlopInit::DontCare => write!(w, " 2")?,
        FlipFlopInit::Unknown => {
            // If we wrote neither type nor clock, we might still need init
            // as a positional argument. But if nothing else was written,
            // we can just omit it (unknown is the default).
            if ff.ty.is_some() || ff.clock.is_some() {
                write!(w, " 3")?;
            }
        }
    }

    writeln!(w)?;
    Ok(())
}

fn write_flop<W: fmt::Write>(ff: &FlipFlop, _cmd: &ModelCmd, w: &mut W) -> fmt::Result {
    write!(w, ".flop D={}", ff.input)?;
    write!(w, " Q={}", ff.output)?;

    if let Some(clock) = &ff.clock {
        write!(w, " C={clock}")?;
    }

    match &ff.ty {
        Some(FlipFlopType::Asynchronous) => write!(w, " async")?,
        Some(FlipFlopType::FallingEdge) => write!(w, " negedge")?,
        _ => {}
    }

    match &ff.init {
        FlipFlopInit::Const(true) => write!(w, " init=1")?,
        FlipFlopInit::Const(false) => write!(w, " init=0")?,
        FlipFlopInit::DontCare => write!(w, " init=2")?,
        FlipFlopInit::Unknown => {}
    }

    writeln!(w)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 1.5  Library Gate (.gate)
// ---------------------------------------------------------------------------

fn write_lib_gate<W: fmt::Write>(
    lg: &LibGate,
    cmd: &ModelCmd,
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    write!(w, ".gate {}", lg.name)?;
    for (formal, actual) in &lg.maps {
        write!(w, " {formal}={actual}")?;
    }
    writeln!(w)?;
    write_attrs(cmd, w, flavor)
}

// ---------------------------------------------------------------------------
// 1.6  Library Latch (.mlatch)
// ---------------------------------------------------------------------------

fn write_lib_ff<W: fmt::Write>(
    lf: &LibFlipFlop,
    cmd: &ModelCmd,
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    write!(w, ".mlatch {}", lf.name)?;
    for (formal, actual) in &lf.maps {
        write!(w, " {formal}={actual}")?;
    }

    if let Some(clock) = &lf.clock {
        write!(w, " {clock}")?;
    }

    match &lf.init {
        FlipFlopInit::Const(true) => write!(w, " 1")?,
        FlipFlopInit::Const(false) => write!(w, " 0")?,
        FlipFlopInit::DontCare => write!(w, " 2")?,
        FlipFlopInit::Unknown => {}
    }

    writeln!(w)?;
    write_attrs(cmd, w, flavor)
}

// ---------------------------------------------------------------------------
// 1.7  /  2.5  Subcircuit (.subckt / .subcircuit)
// ---------------------------------------------------------------------------

fn write_submodel<W: fmt::Write>(
    name: &str,
    map: &[(Str<16>, Str<16>)],
    instance_name: Option<&str>,
    cmd: &ModelCmd,
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    let kw = if flavor == BlifFlavor::ABC {
        ".subcircuit"
    } else {
        ".subckt"
    };

    write!(w, "{kw}")?;
    if let Some(inst) = instance_name {
        write!(w, " {name}|{inst}")?;
    } else {
        write!(w, " {name}")?;
    }
    for (formal, actual) in map {
        write!(w, " {formal}={actual}")?;
    }
    writeln!(w)?;
    write_attrs(cmd, w, flavor)
}

// ---------------------------------------------------------------------------
// 3.4 / 4.3  Connection (.conn / .barbuff / .short)
// ---------------------------------------------------------------------------

fn write_connect<W: fmt::Write>(
    from: &str,
    to: &str,
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    match flavor {
        BlifFlavor::SisMV => {
            writeln!(w, ".short {from} {to}")
        }
        BlifFlavor::Yosys => {
            // Yosys usually emits .conn; .barbuff is an alternative
            writeln!(w, ".conn {from} {to}")
        }
        _ => {
            // Not supported by Standard / ABC / Sis — emit as comment
            writeln!(
                w,
                "# .conn {from} {to}  (not representable in {flavor:?} flavor)"
            )
        }
    }
}

// ---------------------------------------------------------------------------
// 1.9  Finite State Machine (.start_kiss … .end_kiss)
// ---------------------------------------------------------------------------

fn write_fsm<W: fmt::Write>(
    fsm: &FSM,
    cmd: &ModelCmd,
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    writeln!(w, ".start_kiss")?;
    writeln!(w, ".i {}", fsm.inputs)?;
    writeln!(w, ".o {}", fsm.outputs)?;
    writeln!(w, ".p {}", fsm.states.len())?;

    // Count unique states for .s
    let mut state_set = std::collections::BTreeSet::new();
    for t in &fsm.states {
        state_set.insert(t.current_state.as_str());
        state_set.insert(t.next_state.as_str());
    }
    writeln!(w, ".s {}", state_set.len())?;

    if let Some(ref reset) = fsm.reset_state {
        writeln!(w, ".r {reset}")?;
    }

    for t in &fsm.states {
        let ins: String = t.input.iter().map(|x: &Tristate| x.to_string()).collect();
        let outs: String = t.output.iter().map(|x: &Tristate| x.to_string()).collect();
        writeln!(w, "{ins} {} {} {outs}", t.current_state, t.next_state)?;
    }

    writeln!(w, ".end_kiss")?;

    if let Some(ref order) = fsm.physical_latch_order {
        write!(w, ".latch_order")?;
        for latch in order {
            write!(w, " {latch}")?;
        }
        writeln!(w)?;
    }

    if let Some(ref assignments) = fsm.state_assignments {
        for (state, code) in assignments {
            let code_str: String = code
                .iter()
                .map(|b: &bool| if *b { "1" } else { "0" })
                .collect();
            writeln!(w, ".code {state} {code_str}")?;
        }
    }

    write_attrs(cmd, w, flavor)
}

// ---------------------------------------------------------------------------
// 1.10  Clock Constraints (.cycle / .clock_event)
// ---------------------------------------------------------------------------

fn write_clock_events<W: fmt::Write>(
    ev: &ClockEvents,
    w: &mut W,
    _flavor: BlifFlavor,
) -> fmt::Result {
    write!(w, ".clock_event {}", ev.percent)?;
    for event in &ev.events {
        let edge = match event.edge {
            ClockEdgeKind::Rise => "r",
            ClockEdgeKind::Fall => "f",
        };
        if let Some((before, after)) = event.before_after {
            write!(w, " ({edge}'{} {before} {after})", event.clock_name)?;
        } else {
            write!(w, " {edge}'{}", event.clock_name)?;
        }
    }
    writeln!(w)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 1.11 / 2.2 / 2.4  Delay and Timing Constraints
// ---------------------------------------------------------------------------

fn write_delay_constraint<W: fmt::Write>(
    dc: &ModelDelayConstraint,
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    match dc {
        ModelDelayConstraint::Input(idc) => {
            // Full 8-argument .delay — core BLIF
            let phase = match idc.phase {
                DelayConstraintPhase::Inverting => "INV",
                DelayConstraintPhase::NonInverting => "NONINV",
                DelayConstraintPhase::Unknown => "UNKNOWN",
            };
            writeln!(
                w,
                ".delay {} {phase} {} {} {} {} {} {}",
                idc.input,
                idc.load,
                idc.max_load,
                idc.block_rise,
                idc.drive_rise,
                idc.block_fall,
                idc.drive_fall,
            )
        }

        ModelDelayConstraint::WireLoadSlope(l) => {
            writeln!(w, ".wire_load_slope {l}")
        }

        ModelDelayConstraint::WireLoads(loads) => {
            write!(w, ".wire")?;
            for l in loads {
                write!(w, " {l}")?;
            }
            writeln!(w)
        }

        ModelDelayConstraint::InputArrivalTime(sat) => {
            write!(w, ".input_arrival {} {} {}", sat.signal, sat.rise, sat.fall)?;
            if let Some(ref rel) = sat.event_relative {
                let ba = match rel.ba {
                    BeforeAfter::Before => 'b',
                    BeforeAfter::After => 'a',
                };
                write!(w, " {ba} {}", rel.event)?;
            }
            writeln!(w)
        }

        ModelDelayConstraint::OutputRequiredTime(sat) => {
            write!(
                w,
                ".output_required {} {} {}",
                sat.signal, sat.rise, sat.fall
            )?;
            if let Some(ref rel) = sat.event_relative {
                let ba = match rel.ba {
                    BeforeAfter::Before => 'b',
                    BeforeAfter::After => 'a',
                };
                write!(w, " {ba} {}", rel.event)?;
            }
            writeln!(w)
        }

        ModelDelayConstraint::DefaultInputArrivalTime((r, f)) => {
            writeln!(w, ".default_input_arrival {r} {f}")
        }

        ModelDelayConstraint::DefaultOutputRequiredTime((r, f)) => {
            writeln!(w, ".default_output_required {r} {f}")
        }

        ModelDelayConstraint::InputDrive(sd) => {
            writeln!(w, ".input_drive {} {} {}", sd.signal, sd.rise, sd.fall)
        }

        ModelDelayConstraint::DefaultInputDrive((r, f)) => {
            writeln!(w, ".default_input_drive {r} {f}")
        }

        ModelDelayConstraint::MaxInputLoad(sl) => {
            writeln!(w, ".max_input_load {} {}", sl.signal, sl.load)
        }

        ModelDelayConstraint::DefaultMaxInputLoad(l) => {
            writeln!(w, ".default_max_input_load {l}")
        }

        ModelDelayConstraint::OutputLoad(sl) => {
            writeln!(w, ".output_load {} {}", sl.signal, sl.load)
        }

        ModelDelayConstraint::DefaultOutputLoad(l) => {
            writeln!(w, ".default_output_load {l}")
        }

        ModelDelayConstraint::AndGateDelay(d) => {
            if flavor == BlifFlavor::ABC {
                writeln!(w, ".and_gate_delay {d}")
            } else {
                // Standard BLIF uses .delay for global delay
                writeln!(w, ".delay {d}")
            }
        }

        ModelDelayConstraint::InputRequired(sl) => {
            if flavor == BlifFlavor::ABC {
                writeln!(w, ".input_required {} {}", sl.signal, sl.load)
            } else {
                // Standard core BLIF can use .delay <signal> <float>
                writeln!(w, ".delay {} {}", sl.signal, sl.load)
            }
        }

        ModelDelayConstraint::OutputArrival(sl) => {
            if flavor == BlifFlavor::ABC {
                writeln!(w, ".output_arrival {} {}", sl.signal, sl.load)
            } else {
                writeln!(
                    w,
                    "# .output_arrival {} {}  (ABC extension)",
                    sl.signal, sl.load
                )
            }
        }

        ModelDelayConstraint::DelayPerPair {
            in_sig,
            out_sig,
            delay,
        } => {
            if flavor == BlifFlavor::ABC {
                writeln!(w, ".delay {in_sig} {out_sig} {delay}")
            } else {
                writeln!(
                    w,
                    "# .delay {in_sig} {out_sig} {delay}  (ABC per-pair extension)"
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4.5  BLIF-MV: .constraint
// ---------------------------------------------------------------------------

fn write_constraint<W: fmt::Write>(
    signals: &[Str<16>],
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    if matches!(flavor, BlifFlavor::SisMV) {
        write!(w, ".constraint")?;
        for sig in signals {
            write!(w, " {sig}")?;
        }
        writeln!(w)
    } else {
        writeln!(
            w,
            "# .constraint {} ...  (BLIF-MV extension)",
            signals.first().map(|s| s.as_str()).unwrap_or("")
        )
    }
}

// ---------------------------------------------------------------------------
// 4.7  BLIF-MV: .onehot
// ---------------------------------------------------------------------------

fn write_onehot<W: fmt::Write>(signals: &[Str<16>], w: &mut W, flavor: BlifFlavor) -> fmt::Result {
    if matches!(flavor, BlifFlavor::SisMV) {
        write!(w, ".onehot")?;
        for sig in signals {
            write!(w, " {sig}")?;
        }
        writeln!(w)
    } else {
        writeln!(
            w,
            "# .onehot {} ...  (BLIF-MV extension)",
            signals.first().map(|s| s.as_str()).unwrap_or("")
        )
    }
}

// ---------------------------------------------------------------------------
// 4.6  BLIF-MV: .reset
// ---------------------------------------------------------------------------

fn write_reset<W: fmt::Write>(
    signal: &str,
    value: &[Tristate],
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    if matches!(flavor, BlifFlavor::SisMV) {
        writeln!(w, ".reset {signal}")?;
        let val_str: String = value.iter().map(|t| t.to_string()).collect();
        writeln!(w, "{val_str}")
    } else {
        let val_str: String = value.iter().map(|t| t.to_string()).collect();
        writeln!(w, "# .reset {signal} {val_str}  (BLIF-MV extension)")
    }
}

// ---------------------------------------------------------------------------
// 4.8  BLIF-MV: .ltlformula
// ---------------------------------------------------------------------------

fn write_ltlformula<W: fmt::Write>(formula: &str, w: &mut W, flavor: BlifFlavor) -> fmt::Result {
    if matches!(flavor, BlifFlavor::SisMV) {
        writeln!(w, ".ltlformula \"{formula}\"")
    } else {
        writeln!(w, "# .ltlformula \"{formula}\"  (BLIF-MV extension)")
    }
}

// ---------------------------------------------------------------------------
// 4.4  BLIF-MV: .spec
// ---------------------------------------------------------------------------

fn write_spec<W: fmt::Write>(filename: &str, w: &mut W, flavor: BlifFlavor) -> fmt::Result {
    if matches!(flavor, BlifFlavor::SisMV) {
        writeln!(w, ".spec {filename}")
    } else {
        writeln!(w, "# .spec {filename}  (BLIF-MV extension)")
    }
}

// ---------------------------------------------------------------------------
// 3.3 / Yosys & BLIF-MV: .gateinit
// ---------------------------------------------------------------------------

fn write_gateinit<W: fmt::Write>(
    signal: &str,
    value: &FlipFlopInit,
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    if matches!(flavor, BlifFlavor::SisMV | BlifFlavor::Yosys) {
        let val_str = match value {
            FlipFlopInit::Const(true) => "1",
            FlipFlopInit::Const(false) => "0",
            FlipFlopInit::DontCare => "2",
            FlipFlopInit::Unknown => "3",
        };
        writeln!(w, ".gateinit {signal}={val_str}")
    } else {
        let val_str = match value {
            FlipFlopInit::Const(true) => "1",
            FlipFlopInit::Const(false) => "0",
            FlipFlopInit::DontCare => "2",
            FlipFlopInit::Unknown => "3",
        };
        writeln!(
            w,
            "# .gateinit {signal}={val_str}  (Yosys/BLIF-MV extension)"
        )
    }
}

// ---------------------------------------------------------------------------
// BLIF-MV: .mv <var> ... <nvalues> [<val-name> ...]
// ---------------------------------------------------------------------------

fn write_mv<W: fmt::Write>(
    variables: &[Str<16>],
    nvalues: usize,
    value_names: &[String],
    w: &mut W,
    flavor: BlifFlavor,
) -> fmt::Result {
    if matches!(flavor, BlifFlavor::SisMV) {
        write!(w, ".mv")?;
        for var in variables {
            write!(w, " {var}")?;
        }
        write!(w, " {nvalues}")?;
        for vn in value_names {
            write!(w, " {vn}")?;
        }
        writeln!(w)
    } else {
        writeln!(w, "# .mv ...  (BLIF-MV extension)")
    }
}
