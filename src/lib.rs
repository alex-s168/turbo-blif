use smallvec::SmallVec;
use std::{iter::Peekable, str::FromStr};

pub mod ast;

pub type Str<const N: usize> = smallstr::SmallString<[u8; N]>;

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct GateMeta {
    pub inputs: Vec<Str<16>>,
    pub output: Str<16>,
    /// external don't care
    pub external_dc: bool,
}

/// truth table consumer
pub trait GateLutConsumer {
    /// according to the spec, out is only ever binary
    fn entry(&mut self, ins: SmallVec<[Tristate; 8]>, out: bool);
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct FlipFlop {
    /// if not set, is a generic DFF
    pub ty: Option<FlipFlopType>,
    pub input: Str<16>,
    pub output: Str<16>,
    /// If a latch does not have a controlling clock speciﬁed, it is assumed that it is actually controlled by a single
    /// global clock. The behavior of this global clock may be interpreted differently by the various algorithms that may
    /// manipulate the model after the model has been read in.
    pub clock: Option<Str<16>>,
    pub init: FlipFlopInit,
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct LibGate {
    /// name of the gate in the technology library
    pub name: Str<16>,
    /// mappings of wires from technology library gate -> rest of the circuit
    ///
    /// from the spec:
    /// > All of the formal parameters of name must be specified in the formal-actual-list and the single output of [name] must be the last one in the list
    pub maps: Vec<(Str<16>, Str<16>)>,
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct LibFlipFlop {
    /// name of the gate in the technology library
    pub name: Str<16>,
    /// mappings of wires from technology library gate -> rest of the circuit
    ///
    /// from the spec:
    /// > All of the formal parameters of name must be specified in the formal-actual-list and the single output of [name] must be the last one in the list
    pub maps: Vec<(Str<16>, Str<16>)>,
    /// If a latch does not have a controlling clock speciﬁed, it is assumed that it is actually controlled by a single
    /// global clock. The behavior of this global clock may be interpreted differently by the various algorithms that may
    /// manipulate the model after the model has been read in.
    pub clock: Option<Str<16>>,
    pub init: FlipFlopInit,
}

pub trait FSMConsumer {
    fn add_transition(&mut self, transition: FSMTransition);
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub enum CellAttr<'a> {
    /// non-standard; emitted by: yosys
    CellName(&'a str),

    /// non-standard; possibly emitted by: yosys
    ///
    /// example: Attr { key: "src", val: "\"some/file.v:320.20-320.28\"" }
    Attr { key: &'a str, val: &'a str },

    /// non-standard; emitted by: yosys
    ///
    /// example: Param { key: "A_WIDTH", val: "00000000000000000000000000000001" }
    Param { key: &'a str, val: &'a str },
}

pub trait CommandConsumer {
    type Gate: GateLutConsumer;
    type FSM: FSMConsumer;

    fn gate(&self, gate: GateMeta) -> Self::Gate;
    fn gate_done(&mut self, gate: Self::Gate);

    fn fsm(&self, inputs: usize, outputs: usize, reset_state: Option<&str>) -> Self::FSM;
    fn fsm_done(
        &mut self,
        fsm: Self::FSM,
        physical_latch_order: Option<Vec<String>>,
        state_assignments: Option<Vec<(String, SmallVec<[bool; 8]>)>>,
    );

    fn ff(&mut self, ff: FlipFlop);
    fn lib_gate(&mut self, gate: LibGate);
    fn lib_ff(&mut self, ff: LibFlipFlop);
    /// copies the whole circuit of the referenced model, and maps the ins/outs/clocks according to
    /// [map]
    fn sub_model(&mut self, model: &str, map: Vec<(Str<16>, Str<16>)>);

    /// attach attr to last gate / fsm / ff / libgate / libff / sub_model
    fn attr(&mut self, attr: CellAttr);

    /// non-standard
    fn connect(&mut self, from: &str, to: &str);
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct ModelMeta {
    pub name: Str<32>,
    /// if [None], HAS TO be inferred from netlist (signals which are not outputs from other blocks)
    pub inputs: Option<Vec<Str<16>>>,
    /// if [None], HAS TO be inferred from netlist (signals which are not inputs to other blocks)
    pub outputs: Option<Vec<Str<16>>>,
    pub clocks: Vec<Str<16>>,
}

pub trait ModelConsumer {
    type Inner: CommandConsumer;

    fn model(&self, meta: ModelMeta) -> Self::Inner;
    fn model_done(&mut self, model: Self::Inner);

    // search the given BLIF file for additional model declarations
    fn search(&mut self, path: &str);
}

fn parse_padding(lines: &mut Peekable<impl Iterator<Item = impl AsRef<str>>>) {
    while lines
        .peek()
        .is_some_and(|x| x.as_ref().trim().is_empty() || x.as_ref().trim().starts_with('#'))
    {
        let _ = lines.next();
    }
}

#[derive(Debug)]
pub enum BlifParserError {
    UnknownKw(String),
    MissingArgs,
    TooManyArgs,
    Invalid,
    UnexpectedEnd,
}

enum AsRefOrString<T> {
    AsRef(T),
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

fn before_cmt(s: &str) -> &str {
    s.split('#').next().unwrap()
}

#[test]
fn test_before_cmt() {
    assert_eq!(before_cmt("aa asa as a # comment"), "aa asa as a ");
}

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

fn is_kw(lines: &mut Peekable<impl Iterator<Item = impl AsRef<str>>>, kw: &str) -> bool {
    lines
        .peek()
        .is_some_and(|x| x.as_ref().split(' ').next().is_some_and(|y| y == kw))
}

#[repr(u8)]
#[derive(Clone, Hash, PartialEq, PartialOrd)]
pub enum Tristate {
    False = 0,
    True = 1,
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

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub enum FlipFlopType {
    FallingEdge,
    RisingEdge,
    ActiveHigh,
    ActiveLow,
    Asynchronous,
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub enum FlipFlopInit {
    Const(bool),
    DontCare,
    Unknown,
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct FSMTransition<'s> {
    pub input: SmallVec<[Tristate; 8]>,
    pub current_state: &'s str,
    pub next_state: &'s str,
    pub output: SmallVec<[Tristate; 8]>,
}

fn str_to_tristates<C: FromIterator<Tristate>>(s: &str) -> Result<C, ()> {
    s.bytes()
        .map(|x| {
            let x = [x];
            let x = unsafe { str::from_utf8_unchecked(&x) };
            x.parse()
        })
        .collect::<Result<_, _>>()
}

fn parse_mod(
    name: &str,
    consumer: &mut impl ModelConsumer,
    lines: &mut Peekable<impl Iterator<Item = impl AsRef<str>>>,
) -> Result<(), BlifParserError> {
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

        let mut args = line.split(' ');
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
                        "0" => false,
                        "1" => true,
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

                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }

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
                let name = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                let maps = args
                    .map(|x| {
                        x.split_once('=')
                            .ok_or(BlifParserError::Invalid)
                            .map(|(k, v)| (k.into(), v.into()))
                    })
                    .collect::<Result<_, _>>()?;

                consumer.sub_model(name, maps);
            }

            ".search" => {
                let path = args.next().ok_or(BlifParserError::MissingArgs)?.into();

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

                let mut fsm =
                    consumer.fsm(num_ins, num_outs, reset_state.as_ref().map(|x| x.as_str()));

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
                    if code_mapping.len() == 0 {
                        None
                    } else {
                        Some(code_mapping)
                    },
                );
            }

            ".cname" => {
                let arg = args.next().ok_or(BlifParserError::MissingArgs)?.into();

                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }

                consumer.attr(CellAttr::CellName(arg));
            }

            ".attr" => {
                let key = args.next().ok_or(BlifParserError::MissingArgs)?.into();

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
                let key = args.next().ok_or(BlifParserError::MissingArgs)?.into();

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

            ".barbuff" | ".conn" => {
                let from = args.next().ok_or(BlifParserError::MissingArgs)?.into();
                let to = args.next().ok_or(BlifParserError::MissingArgs)?.into();

                if args.next().is_some() {
                    Err(BlifParserError::TooManyArgs)?
                }

                consumer.connect(from, to);
            }

            // TODO: clock & delay cst
            // TODO: .blackblox
            _ => Err(BlifParserError::UnknownKw(cmd.to_string()))?,
        };
    }

    main_consumer.model_done(consumer);

    Ok(())
}

/// # Arguments
///
/// * `file_name` - when the model name is not declared explicitly in the BLIF file, this is used as model name (compliant with specification)
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
                    let path = args.next().ok_or(BlifParserError::MissingArgs)?.into();

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
