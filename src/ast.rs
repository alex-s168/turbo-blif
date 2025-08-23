use std::path::Path;

use super::*;

#[derive(Clone, Hash, PartialEq, PartialOrd)]
pub struct LUT(pub Vec<(SmallVec<[Tristate; 8]>, bool)>);

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
                if ent.1 { "1" } else { "0" }
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
    fn entry(&mut self, ins: SmallVec<[Tristate; 8]>, out: bool) {
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
pub enum ModelCmd {
    Gate(Gate),
    FF(FlipFlop),
    LibGate(LibGate),
    LibFF(LibFlipFlop),
    FSM(FSM),
    SubModel {
        name: Str<32>,
        map: Vec<(Str<16>, Str<16>)>,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd)]
pub struct Model {
    pub meta: ModelMeta,
    pub commands: Vec<ModelCmd>,
}

impl CommandConsumer for Model {
    type Gate = Gate;

    fn gate(&self, gate: GateMeta) -> Self::Gate {
        gate.into()
    }

    fn gate_done(&mut self, gate: Self::Gate) {
        self.commands.push(ModelCmd::Gate(gate));
    }

    fn ff(&mut self, ff: FlipFlop) {
        self.commands.push(ModelCmd::FF(ff));
    }

    fn lib_gate(&mut self, gate: LibGate) {
        self.commands.push(ModelCmd::LibGate(gate));
    }

    fn lib_ff(&mut self, ff: LibFlipFlop) {
        self.commands.push(ModelCmd::LibFF(ff));
    }

    fn sub_model(&mut self, model: &str, map: Vec<(Str<16>, Str<16>)>) {
        self.commands.push(ModelCmd::SubModel {
            name: model.into(),
            map,
        });
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
        self.commands.push(ModelCmd::FSM(fsm));
    }
}

#[derive(Debug)]
pub enum FullBlifErr<E: std::fmt::Debug> {
    Blif(BlifParserError),
    File(E),
    FileNoName,
    /// only caused when parsing single blif file
    SearchPathsNotSupported,
}

#[derive(Debug, PartialEq, Hash)]
pub enum BlifEntry {
    Model(Model),
}

#[derive(Debug, PartialEq, Hash)]
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
