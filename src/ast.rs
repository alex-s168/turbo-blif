use std::path::Path;

use super::*;

#[derive(Clone, Hash, PartialEq, PartialOrd)]
pub struct LUT(pub Vec<(SmallVec<[Tristate; 8]>, Option<bool>)>);

impl std::fmt::Display for LUT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ent in self.0.iter() {
            write!(
                f,
                "{} {}\n",
                ent.0.iter().fold(String::new(), |acc, x| {
                    let mut acc = acc;
                    acc.push_str(x.to_string().as_str());
                    acc
                }),
                match ent.1 {
                    Some(true) => "1",
                    Some(false) => "0",
                    None => "-",
                }
            )?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for LUT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", format!("{}", self))
    }
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct Gate {
    pub meta: GateMeta,
    pub lut: LUT,
}

impl From<GateMeta> for Gate {
    fn from(value: GateMeta) -> Self {
        Self {
            meta: value,
            lut: LUT(vec![]),
        }
    }
}

impl GateLutConsumer for Gate {
    fn entry(&mut self, ins: SmallVec<[Tristate; 8]>, out: Option<bool>) {
        self.lut.0.push((ins, out));
    }
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct FSMTransitionAST {
    pub input: SmallVec<[Tristate; 8]>,
    pub current_state: String,
    pub next_state: String,
    pub output: SmallVec<[Tristate; 8]>,
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct FSM {
    pub inputs: usize,
    pub outputs: usize,
    pub reset_state: Option<String>,
    pub states: Vec<FSMTransitionAST>,
    pub physical_latch_order: Option<Vec<String>>,
    pub state_assignments: Option<Vec<(String, SmallVec<[bool; 8]>)>>,
}

impl FSMConsumer for FSM {
    fn add_transition(&mut self, transition: FSMTransition) {
        self.states.push(FSMTransitionAST {
            input: transition.input,
            current_state: transition.current_state.to_string(),
            next_state: transition.next_state.to_string(),
            output: transition.output,
        });
    }
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub enum CellAttrAst {
    /// non-standard; emitted by: yosys
    CellName(String),

    /// non-standard; possibly emitted by: yosys
    ///
    /// example: Attr { key: "src", val: "\"some/file.v:320.20-320.28\"" }
    Attr { key: Str<8>, val: String },

    /// non-standard; emitted by: yosys
    ///
    /// example: Param { key: "A_WIDTH", val: "00000000000000000000000000000001" }
    Param { key: Str<16>, val: String },
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ModelCmdKind {
    Gate(Gate),
    FF(FlipFlop),
    LibGate(LibGate),
    LibFF(LibFlipFlop),
    FSM(FSM),
    SubModel {
        name: Str<32>,
        map: Vec<(Str<16>, Str<16>)>,
    },
    Connect {
        from: Str<16>,
        to: Str<16>,
    },
    CycleTime(f32),
    ClockEvents(ClockEvents),
    DelayConstraint(ModelDelayConstraint),
    /// BLIF-MV: `.constraint <signal>` (`signal` can appear multiple times)
    Constraint(Vec<Str<16>>),
    /// BLIF-MV: `.onehot <signal>` (`signal` can appear multiple times)
    OneHot(Vec<Str<16>>),
    /// BLIF-MV: `.reset <signal> \n <value>`
    Reset {
        signal: Str<16>,
        value: SmallVec<[Tristate; 8]>,
    },
    /// BLIF-MV: `.ltlformula "<LTL string>"`
    LtlFormula(String),
    /// BLIF-MV: `.spec <file-name>`
    Spec(String),
    /// BLIF-MV / Yosys: `.gateinit <signal>=<init-val>`
    GateInit {
        signal: Str<16>,
        value: FlipFlopInit,
    },
    /// BLIF-MV: `.mv <var> ... <nvalues> [<val-name> ...]`
    Mv {
        variables: Vec<Str<16>>,
        nvalues: usize,
        value_names: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ModelCmd {
    pub kind: ModelCmdKind,
    pub attrs: Vec<CellAttrAst>,
}

impl From<ModelCmdKind> for ModelCmd {
    fn from(value: ModelCmdKind) -> Self {
        Self {
            kind: value,
            attrs: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct ModelAttr {
    /// from blif `.area` attribute
    pub area: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Model {
    pub meta: ModelMeta,
    pub commands: Vec<ModelCmd>,
    pub attr: ModelAttr,
}

impl CommandConsumer for Model {
    type Gate = Gate;

    fn gate(&self, gate: GateMeta) -> Self::Gate {
        gate.into()
    }

    fn gate_done(&mut self, gate: Self::Gate) {
        self.commands.push(ModelCmdKind::Gate(gate).into());
    }

    fn ff(&mut self, ff: FlipFlop) {
        self.commands.push(ModelCmdKind::FF(ff).into());
    }

    fn lib_gate(&mut self, gate: LibGate) {
        self.commands.push(ModelCmdKind::LibGate(gate).into());
    }

    fn lib_ff(&mut self, ff: LibFlipFlop) {
        self.commands.push(ModelCmdKind::LibFF(ff).into());
    }

    fn sub_model(&mut self, model: &str, map: Vec<(Str<16>, Str<16>)>) {
        self.commands.push(
            ModelCmdKind::SubModel {
                name: model.into(),
                map,
            }
            .into(),
        );
    }

    type FSM = FSM;

    fn fsm(&self, inputs: usize, outputs: usize, reset_state: Option<&str>) -> Self::FSM {
        FSM {
            inputs,
            outputs,
            reset_state: reset_state.map(|x| x.to_string()),
            states: vec![],
            physical_latch_order: None,
            state_assignments: None,
        }
    }

    fn fsm_done(
        &mut self,
        fsm: Self::FSM,
        physical_latch_order: Option<Vec<String>>,
        state_assignments: Option<Vec<(String, SmallVec<[bool; 8]>)>>,
    ) {
        let mut fsm = fsm;
        fsm.physical_latch_order = physical_latch_order;
        fsm.state_assignments = state_assignments;
        self.commands.push(ModelCmdKind::FSM(fsm).into());
    }

    fn attr(&mut self, attr: CellAttr) {
        self.commands.last_mut().unwrap().attrs.push(match attr {
            CellAttr::CellName(n) => CellAttrAst::CellName(n.into()),
            CellAttr::Attr { key, val } => CellAttrAst::Attr {
                key: key.into(),
                val: val.into(),
            },
            CellAttr::Param { key, val } => CellAttrAst::Param {
                key: key.into(),
                val: val.into(),
            },
        });
    }

    fn connect(&mut self, from: &str, to: &str) {
        self.commands.push(
            ModelCmdKind::Connect {
                from: from.into(),
                to: to.into(),
            }
            .into(),
        )
    }

    fn set_area(&mut self, area: f64) {
        self.attr.area = Some(area);
    }

    fn set_cycle_time(&mut self, cycle_time: f32) {
        self.commands
            .push(ModelCmdKind::CycleTime(cycle_time).into())
    }

    fn clock_events(&mut self, events: ClockEvents) {
        self.commands.push(ModelCmdKind::ClockEvents(events).into())
    }

    fn model_delay_constraint(&mut self, constraint: ModelDelayConstraint) {
        self.commands
            .push(ModelCmdKind::DelayConstraint(constraint).into())
    }

    fn constraint(&mut self, signals: &[Str<16>]) {
        self.commands
            .push(ModelCmdKind::Constraint(signals.to_vec()).into());
    }

    fn onehot(&mut self, signals: &[Str<16>]) {
        self.commands
            .push(ModelCmdKind::OneHot(signals.to_vec()).into());
    }

    fn reset(&mut self, signal: Str<16>, value: SmallVec<[Tristate; 8]>) {
        self.commands
            .push(ModelCmdKind::Reset { signal, value }.into());
    }

    fn ltlformula(&mut self, formula: &str) {
        self.commands
            .push(ModelCmdKind::LtlFormula(formula.to_string()).into());
    }

    fn spec(&mut self, filename: &str) {
        self.commands
            .push(ModelCmdKind::Spec(filename.to_string()).into());
    }

    fn gateinit(&mut self, signal: Str<16>, value: FlipFlopInit) {
        self.commands
            .push(ModelCmdKind::GateInit { signal, value }.into());
    }

    fn mv(&mut self, variables: Vec<Str<16>>, nvalues: usize, value_names: Vec<String>) {
        self.commands.push(
            ModelCmdKind::Mv {
                variables,
                nvalues,
                value_names,
            }
            .into(),
        );
    }
}

#[derive(Debug)]
pub enum FullBlifErr<E: std::fmt::Debug> {
    Blif(BlifParserError),
    File(E),
    FileNoName,
    /// Only caused when parsing a single BLIF file and a `.search` directive is encountered.
    SearchPathsNotSupported,
}

impl<E: std::fmt::Debug + std::fmt::Display> std::fmt::Display for FullBlifErr<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FullBlifErr::Blif(e) => write!(f, "BLIF parse error: {e}"),
            FullBlifErr::File(e) => write!(f, "I/O error: {e}"),
            FullBlifErr::FileNoName => write!(f, "file path has no file name"),
            FullBlifErr::SearchPathsNotSupported => {
                write!(
                    f,
                    ".search directives are not supported when parsing a single BLIF file"
                )
            }
        }
    }
}

impl<E: std::fmt::Debug + std::fmt::Display> std::error::Error for FullBlifErr<E> {}

#[derive(Debug, PartialEq)]
pub enum BlifEntry {
    Model(Model),
}

#[derive(Debug, PartialEq)]
pub struct Blif {
    pub entries: Vec<BlifEntry>,
    to_search: Vec<String>,
}

impl ModelConsumer for Blif {
    type Inner = Model;

    fn model(&self, meta: ModelMeta) -> Self::Inner {
        Model {
            meta,
            commands: vec![],
            attr: Default::default(),
        }
    }

    fn model_done(&mut self, model: Self::Inner) {
        self.entries.push(BlifEntry::Model(model));
    }

    fn search(&mut self, path: &str) {
        self.to_search.push(path.to_string());
    }
}

pub fn parse_many_blif_to_ast<E: std::fmt::Debug, L: IntoIterator<Item = impl AsRef<str>>>(
    path: &str,
    lut: impl Fn(&str) -> Result<L, E>,
) -> Result<Blif, FullBlifErr<E>> {
    let mut blif = Blif {
        entries: vec![],
        to_search: vec![path.to_string()],
    };

    while !blif.to_search.is_empty() {
        let p = blif.to_search.pop().unwrap();
        let filnam = Path::new(p.as_str())
            .file_name()
            .ok_or(FullBlifErr::FileNoName)?;
        let filnam = filnam.to_string_lossy();
        let p = lut(p.as_str()).map_err(FullBlifErr::File)?;
        parse_blif(filnam.as_ref(), &mut blif, p).map_err(FullBlifErr::Blif)?;
    }

    Ok(blif)
}

pub fn parse_blif_to_ast(
    filename: &str,
    lines: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<Blif, FullBlifErr<()>> {
    let mut blif = Blif {
        entries: vec![],
        to_search: vec![],
    };

    parse_blif(filename, &mut blif, lines).map_err(FullBlifErr::Blif)?;

    if !blif.to_search.is_empty() {
        Err(FullBlifErr::SearchPathsNotSupported)?;
    }

    Ok(blif)
}

pub fn parse_str_blif_to_ast(filename: &str, source: &str) -> Result<Blif, FullBlifErr<()>> {
    parse_blif_to_ast(filename, source.split('\n'))
}
