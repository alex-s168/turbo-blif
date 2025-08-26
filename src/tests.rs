#![allow(non_snake_case)]
#![allow(unused)]

use std::default;

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
            commands: vec![
                ModelCmdKind::Gate(Gate {
                    meta: GateMeta {
                        inputs: vec!["a".into(), "b".into(),],
                        output: "c".into(),
                        external_dc: false,
                    },
                    lut: LUT(vec![(
                        [Tristate::True, Tristate::True].into_iter().collect(),
                        true
                    )]),
                })
                .into()
            ],
            attr: Default::default(),
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
            commands: vec![
                ModelCmdKind::Gate(Gate {
                    meta: GateMeta {
                        inputs: vec!["a".into(), "b".into(),],
                        output: "c".into(),
                        external_dc: false,
                    },
                    lut: LUT(vec![(
                        [Tristate::True, Tristate::True].into_iter().collect(),
                        true
                    )]),
                })
                .into()
            ],
            attr: Default::default(),
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
            commands: vec![
                ModelCmdKind::Gate(Gate {
                    meta: GateMeta {
                        inputs: vec!["a".into(), "b".into(),],
                        output: "c".into(),
                        external_dc: false,
                    },
                    lut: LUT(vec![(
                        [Tristate::True, Tristate::True].into_iter().collect(),
                        true
                    )]),
                })
                .into()
            ],
            attr: Default::default(),
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
            commands: vec![
                ModelCmdKind::Gate(Gate {
                    meta: GateMeta {
                        inputs: vec!["a".into(), "b".into(),],
                        output: "c".into(),
                        external_dc: true,
                    },
                    lut: LUT(vec![(
                        [Tristate::True, Tristate::True].into_iter().collect(),
                        true
                    )]),
                })
                .into()
            ],
            attr: Default::default(),
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
            commands: vec![
                ModelCmdKind::Gate(Gate {
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
                })
                .into()
            ],
            attr: Default::default(),
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
                    ModelCmdKind::SubModel {
                        name: "b".into(),
                        map: vec![
                            ("x".into(), "x".into()),
                            ("y".into(), "y".into()),
                            ("j".into(), "j".into())
                        ]
                    }
                    .into(),
                    ModelCmdKind::Gate(Gate {
                        meta: GateMeta {
                            inputs: vec!["x".into()],
                            output: "j".into(),
                            external_dc: true
                        },
                        lut: LUT(vec![([Tristate::True].into_iter().collect(), true)])
                    })
                    .into()
                ],
                attr: Default::default(),
            }),
            BlifEntry::Model(Model {
                meta: ModelMeta {
                    name: "b".into(),
                    inputs: Some(vec!["x".into(), "y".into()]),
                    outputs: Some(vec!["j".into()]),
                    clocks: vec![]
                },
                commands: vec![
                    ModelCmdKind::Gate(Gate {
                        meta: GateMeta {
                            inputs: vec!["x".into(), "y".into()],
                            output: "j".into(),
                            external_dc: false
                        },
                        lut: LUT(vec![(
                            [Tristate::True, Tristate::True].into_iter().collect(),
                            true
                        )])
                    })
                    .into()
                ],
                attr: Default::default(),
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
fn clock_cst() {
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

#[test]
fn yosys_attrs() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"

.model MAC
.inputs clock reset io_rmii_r_RXD[0] io_rmii_r_RXD[1] io_rmii_r_RXER io_rmii_r_CRS_DV io_rmii_clk io_is10mbits io_full_duplex xmit_io_transmit xmit_io_byte[0] xmit_io_byte[1] xmit_io_byte[2] xmit_io_byte[3] xmit_io_byte[4] xmit_io_byte[5] xmit_io_byte[6] xmit_io_byte[7]
.outputs io_rmii_t_TXD[0] io_rmii_t_TXD[1] io_rmii_t_TXEN xmit_io_byte_sent recv_io_frame_start recv_io_abort_frame recv_io_valid_byte recv_io_byte[0] recv_io_byte[1] recv_io_byte[2] recv_io_byte[3] recv_io_byte[4] recv_io_byte[5] recv_io_byte[6] recv_io_byte[7] recv_io_info_carrier_lost_during_packet
.names $false
.names $true
1
.names $undef

.subckt $add A[0]=xmit_preamble_counter[0] A[1]=xmit_preamble_counter[1] A[2]=xmit_preamble_counter[2] B[0]=$true B[1]=$false B[2]=$false Y[0]=$add$build/out.v:213$277_Y[0] Y[1]=$add$build/out.v:213$277_Y[1] Y[2]=$add$build/out.v:213$277_Y[2]
.cname $add$build/out.v:213$277
.attr src "build/out.v:213.31-213.59"
.param A_SIGNED 00000000000000000000000000000000
.param A_WIDTH 00000000000000000000000000000011
.param B_SIGNED 00000000000000000000000000000000
.param B_WIDTH 00000000000000000000000000000011
.param Y_WIDTH 00000000000000000000000000000011

.subckt $and A=$not$build/out.v:170$224_Y B=$eq$build/out.v:170$225_Y Y=$and$build/out.v:170$226_Y
.cname $and$build/out.v:170$226
.attr src "build/out.v:170.31-170.103"
.param A_SIGNED 00000000000000000000000000000000
.param A_WIDTH 00000000000000000000000000000001
.param B_SIGNED 00000000000000000000000000000000
.param B_WIDTH 00000000000000000000000000000001
.param Y_WIDTH 00000000000000000000000000000001


"#,
    )
    .unwrap();

    // TODO: assert_eq
}

#[test]
fn delay_cst__area() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.names $true
1
# .area is not a model attribute, but a command
.area 100.31
"#,
    )
    .unwrap();

    assert_eq!(
        ast.entries,
        vec![BlifEntry::Model(Model {
            meta: ModelMeta {
                name: "top".into(),
                inputs: None,
                outputs: None,
                clocks: vec![],
            },
            commands: vec![ModelCmd {
                kind: ModelCmdKind::Gate(Gate {
                    meta: GateMeta {
                        inputs: vec![],
                        output: "$true".into(),
                        external_dc: false,
                    },
                    lut: LUT(vec![([].into_iter().collect(), true)])
                }),
                attrs: vec![],
            }],
            attr: ModelAttr {
                area: Some(100.31),
                ..Default::default()
            }
        })]
    );
}

/// generated by abc: https://github.com/berkeley-abc/abc/blob/master/src/base/io/ioWriteBlif.c
#[test]
fn abc__and_gate_delay() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.names $true
1
# not a model attribute, but a command
.and_gate_delay 12.31
"#,
    )
    .unwrap();

    // TODO: assert
}

#[test]
fn delay_cst__default_spec() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.names $true
1
# not a model attribute, but a command

.default_input_arrival   3.0  3.2
.default_output_required 2.1  2.5

.default_max_input_load  200
.default_output_load     300
"#,
    )
    .unwrap();

    // TODO: assert
}

// TODO:
//.delay <in-name> <phase> <load> <max-load> <brise> <drise> <bfall> <dfall>
//.wire_load_slope <load>
//.wire <wire-load-list>
//.input_arrival <in-name> <rise> <fall> [<before-after> <event>]
//.output_required <out-name> <rise> <fall> [<before-after> <event>]
//.max_input_load <load>
//.input_drive <in-name> <rise> <fall>
//.output_load <out-name> <load>
