use super::ast::*;
use super::*;

#[test]
fn simple_named() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model simple
.inputs a b
.outputs c
.names a b c # .names described later
11 1
.end
"#,
    )
    .unwrap();
    assert_eq!(
        ast.entries,
        vec![BlifEntry::Model(Model {
            meta: ModelMeta {
                name: "simple".into(),
                inputs: Some(vec!["a".into(), "b".into()]),
                outputs: Some(vec!["c".into()]),
                clocks: vec![],
            },
            commands: vec![ModelCmd::Gate(Gate {
                meta: GateMeta {
                    inputs: vec!["a".into(), "b".into(),],
                    output: "c".into(),
                    external_dc: false,
                },
                lut: LUT(vec![(
                    [Tristate::True, Tristate::True].into_iter().collect(),
                    true
                )]),
            })],
        })],
    );
}

#[test]
fn simple_unnamed() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.inputs a b
.outputs c
.names a b c # .names described later
11 1
"#,
    )
    .unwrap();
    assert_eq!(
        ast.entries,
        vec![BlifEntry::Model(Model {
            meta: ModelMeta {
                name: "top.blif".into(),
                inputs: Some(vec!["a".into(), "b".into()]),
                outputs: Some(vec!["c".into()]),
                clocks: vec![],
            },
            commands: vec![ModelCmd::Gate(Gate {
                meta: GateMeta {
                    inputs: vec!["a".into(), "b".into(),],
                    output: "c".into(),
                    external_dc: false,
                },
                lut: LUT(vec![(
                    [Tristate::True, Tristate::True].into_iter().collect(),
                    true
                )]),
            })],
        })],
    );
}

#[test]
fn simple_unnamed_infer() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.names a b \
c
11 1
"#,
    )
    .unwrap();
    assert_eq!(
        ast.entries,
        vec![BlifEntry::Model(Model {
            meta: ModelMeta {
                name: "top.blif".into(),
                inputs: None,
                outputs: None,
                clocks: vec![],
            },
            commands: vec![ModelCmd::Gate(Gate {
                meta: GateMeta {
                    inputs: vec!["a".into(), "b".into(),],
                    output: "c".into(),
                    external_dc: false,
                },
                lut: LUT(vec![(
                    [Tristate::True, Tristate::True].into_iter().collect(),
                    true
                )]),
            })],
        })],
    );
}

#[test]
fn simple_unnamed_infer_extdc() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.exdc
.names a b \
c
11 1
"#,
    )
    .unwrap();
    assert_eq!(
        ast.entries,
        vec![BlifEntry::Model(Model {
            meta: ModelMeta {
                name: "top.blif".into(),
                inputs: None,
                outputs: None,
                clocks: vec![],
            },
            commands: vec![ModelCmd::Gate(Gate {
                meta: GateMeta {
                    inputs: vec!["a".into(), "b".into(),],
                    output: "c".into(),
                    external_dc: true,
                },
                lut: LUT(vec![(
                    [Tristate::True, Tristate::True].into_iter().collect(),
                    true
                )]),
            })],
        })],
    );
}

#[test]
fn lut() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.names v3 v6 j u78 v13.15
1--0 1
-1-1 1
0-11 1
"#,
    )
    .unwrap();
    assert_eq!(
        ast.entries,
        vec![BlifEntry::Model(Model {
            meta: ModelMeta {
                name: "top.blif".into(),
                inputs: None,
                outputs: None,
                clocks: vec![],
            },
            commands: vec![ModelCmd::Gate(Gate {
                meta: GateMeta {
                    inputs: vec!["v3".into(), "v6".into(), "j".into(), "u78".into()],
                    output: "v13.15".into(),
                    external_dc: false,
                },
                lut: LUT(vec![
                    (
                        [
                            Tristate::True,
                            Tristate::Ignored,
                            Tristate::Ignored,
                            Tristate::False
                        ]
                        .into_iter()
                        .collect(),
                        true
                    ),
                    (
                        [
                            Tristate::Ignored,
                            Tristate::True,
                            Tristate::Ignored,
                            Tristate::True
                        ]
                        .into_iter()
                        .collect(),
                        true
                    ),
                    (
                        [
                            Tristate::False,
                            Tristate::Ignored,
                            Tristate::True,
                            Tristate::True
                        ]
                        .into_iter()
                        .collect(),
                        true
                    )
                ]),
            })],
        })],
    );
}

#[test]
fn submod() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model a
.inputs x y
.outputs j
.subckt b x=x y=y j=j
.exdc
.names x j
1 1
.end


.model b
.inputs x y
.outputs j
.names x y j
11 1
.end
"#,
    )
    .unwrap();
    assert_eq!(
        ast.entries,
        vec![
            BlifEntry::Model(Model {
                meta: ModelMeta {
                    name: "a".into(),
                    inputs: Some(vec!["x".into(), "y".into()]),
                    outputs: Some(vec!["j".into()]),
                    clocks: vec![]
                },
                commands: vec![
                    ModelCmd::SubModel {
                        name: "b".into(),
                        map: vec![
                            ("x".into(), "x".into()),
                            ("y".into(), "y".into()),
                            ("j".into(), "j".into())
                        ]
                    },
                    ModelCmd::Gate(Gate {
                        meta: GateMeta {
                            inputs: vec!["x".into()],
                            output: "j".into(),
                            external_dc: true
                        },
                        lut: LUT(vec![([Tristate::True].into_iter().collect(), true)])
                    })
                ]
            }),
            BlifEntry::Model(Model {
                meta: ModelMeta {
                    name: "b".into(),
                    inputs: Some(vec!["x".into(), "y".into()]),
                    outputs: Some(vec!["j".into()]),
                    clocks: vec![]
                },
                commands: vec![ModelCmd::Gate(Gate {
                    meta: GateMeta {
                        inputs: vec!["x".into(), "y".into()],
                        output: "j".into(),
                        external_dc: false
                    },
                    lut: LUT(vec![(
                        [Tristate::True, Tristate::True].into_iter().collect(),
                        true
                    )])
                })]
            })
        ]
    );
}

#[test]
fn simple_ff() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.inputs in # a very simple sequential circuit
.outputs out
.latch out in 0
.names in out
0 1
.end
"#,
    )
    .unwrap();

    // TODO: assert_eq
}

#[test]
fn simple_ff_clocked() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.inputs d     # a clocked flip-flop
.output q
.clock c
.latch d q re c 0
.end
"#,
    )
    .unwrap();

    // TODO: assert_eq
}

#[test]
fn tech_gate() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.inputs v1 v2
.outputs j
.gate nand2 A=v1 B=v2 O=x # given: formals of this gate are A, B, O
.gate inv A=x O=j # given: formals of this gate are A & O
.end
"#,
    )
    .unwrap();

    // TODO: assert_eq
}

#[test]
fn tech_ff() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.inputs j kbar
.outputs out
.clock clk
.mlatch jk_rising_edge J=j K=k Q=q clk 1 # given: formals are J, K, Q
.names q out
0 1
.names kbar k
0 1
.end
"#,
    )
    .unwrap();

    // TODO: assert_eq
}

#[test]
fn kiss_fsm() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model 101 # outputs 1 whenever last 3 inputs were 1, 0, 1
.start_kiss
.i 1
.o 1
0 st0 st0 0
1 st0 st1 0
0 st1 st2 0
1 st1 st1 0
0 st2 st0 0
1 st2 st3 1
0 st3 st2 0
1 st3 st1 0
.end_kiss
.end
"#,
    )
    .unwrap();

    // TODO: assert_eq
}

#[test]
fn kiss_fsm2() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model
.inputs v0
.outputs v3.2
.latch [6] v1 0
.latch [7] v2 0
.start_kiss
.i 1
.o 1
.p 8
.s 4
.r st0
0 st0 st0 0
1 st0 st1 0
0 st1 st2 0
1 st1 st1 0
0 st2 st0 0
1 st2 st3 1
0 st3 st2 0
1 st3 st1 0
.end_kiss
.latch_order v1 v2
.code st0 00
.code st1 11
.code st2 01
.code st3 10
.names v0 [6]
1 1
.names v0 v1 v2 [7]
-1- 1
1-0 1
.names v0 v1 v2 v3.2
101 1
.end
"#,
    )
    .unwrap();

    // TODO: assert_eq
}

#[test]
fn tech_clock_cst() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.clock clock1 clock2
.clock_event 50.0 r’clock1
.clock_event 50.0 (f’clock2 2.0 5.0)
"#,
    )
    .unwrap();

    // TODO: assert_eq
}
