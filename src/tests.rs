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
                        Some(true)
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
                        Some(true)
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
                        Some(true)
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
                        Some(true)
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
                            Some(true)
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
                            Some(true)
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
                            Some(true)
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
                        ],
                        instance_name: None
                    }
                    .into(),
                    ModelCmdKind::Gate(Gate {
                        meta: GateMeta {
                            inputs: vec!["x".into()],
                            output: "j".into(),
                            external_dc: true
                        },
                        lut: LUT(vec![([Tristate::True].into_iter().collect(), Some(true))])
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
                            Some(true)
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
.clock_event 50.0 r'clock1
.clock_event 50.0 (f'clock2 2.0 5.0)
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
                    lut: LUT(vec![([].into_iter().collect(), Some(true))])
                }),
                attrs: vec![],
            }],
            attr: ModelAttr { area: Some(100.31) }
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

#[test]
fn delay_cst__delay() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a b c
.names $true
1

.delay b NONINV 1.0 2.0 1.0 1.1 1.3 1.1

"#,
    )
    .unwrap();

    // TODO: assert
}

#[test]
fn delay_cst__input_arrival() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a
.names $true
1
.input_arrival a 1.5 2.5
"#,
    )
    .unwrap();
}

#[test]
fn delay_cst__input_arrival_with_event() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a
.clock clk
.names $true
1
.input_arrival a 1.5 2.5 b r'clk
"#,
    )
    .unwrap();
}

#[test]
fn delay_cst__output_required() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.outputs z
.names $true
1
.output_required z 3.0 3.2
"#,
    )
    .unwrap();
}

#[test]
fn delay_cst__input_drive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a
.names $true
1
.input_drive a 0.5 0.6
"#,
    )
    .unwrap();
}

#[test]
fn delay_cst__default_input_drive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.names $true
1
.default_input_drive 0.5 0.6
"#,
    )
    .unwrap();
}

#[test]
fn delay_cst__output_load() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.outputs z
.names $true
1
.output_load z 200.0
"#,
    )
    .unwrap();
}

#[test]
fn delay_cst__default_output_load() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.names $true
1
.default_output_load 300.0
"#,
    )
    .unwrap();
}

#[test]
fn delay_cst__max_input_load() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a
.names $true
1
.max_input_load a 150.0
"#,
    )
    .unwrap();
}

#[test]
fn delay_cst__default_max_input_load() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.names $true
1
.default_max_input_load 150.0
"#,
    )
    .unwrap();
}

#[test]
fn delay_cst__wire_load_slope() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.names $true
1
.wire_load_slope 0.01
"#,
    )
    .unwrap();
}

#[test]
fn delay_cst__wire() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.names $true
1
.wire 0.1 0.2 0.3
"#,
    )
    .unwrap();
}

#[test]
fn blackbox_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a
.outputs z
.blackbox
.end
"#,
    )
    .unwrap();
}

#[test]
fn cover_alternative() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a b
.outputs c
.cover 2 1 1
a b c
11 1
.end
"#,
    )
    .unwrap();
}

#[test]
fn attrib_white_box_comb() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model fa
.inputs a b cin
.outputs s cout
.attrib white box comb
.delay a s 0.01
.delay b s 0.01
.delay cin s 0.01
.names a b cin s
100 1
010 1
001 1
111 1
.end
"#,
    )
    .unwrap();
}

#[test]
fn no_merge_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model fa
.inputs a b cin
.outputs s cout
.no_merge s
.attrib white box comb
.names a b cin s
100 1
010 1
001 1
111 1
.end
"#,
    )
    .unwrap();
}

#[test]
fn flop_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model flop
.inputs d clk rst
.outputs q
.flop D=d C=clk R=rst Q=q async init=1
.end
"#,
    )
    .unwrap();
}

#[test]
fn latch_with_register_class() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs d
.outputs q
.clock clk
.latch d q re clk 0 15
.end
"#,
    )
    .unwrap();
}

#[test]
fn subcircuit_alias() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a b
.outputs z
.subcircuit adder a=a b=b z=z
.end

.model adder
.inputs a b
.outputs z
.names a b z
11 1
.end
"#,
    )
    .unwrap();
}

#[test]
fn names_with_x_output() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a
.outputs z
.names a z
1 x
.end
"#,
    )
    .unwrap();
}

#[test]
fn names_with_n_output() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a
.outputs z
.names a z
1 n
.end
"#,
    )
    .unwrap();
}

#[test]
fn short_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a
.outputs z
.short a z
.end
"#,
    )
    .unwrap();
}

#[test]
fn constraint_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a b
.outputs z
.constraint a b
.names a b z
11 1
.end
"#,
    )
    .unwrap();
}

#[test]
fn onehot_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.latch d1 q1 re clk 0
.latch d2 q2 re clk 0
.latch d3 q3 re clk 0
.onehot q1 q2 q3
.end
"#,
    )
    .unwrap();
}

#[test]
fn reset_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs d rst
.outputs q
.latch d q re clk 0
.reset q
0
.end
"#,
    )
    .unwrap();
}

#[test]
fn ltlformula_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.ltlformula "G(a -> F b)"
.end
"#,
    )
    .unwrap();
}

#[test]
fn mv_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs s
.outputs z
.mv s 3 red green blue
.names s z
1 1
.end
"#,
    )
    .unwrap();
}

#[test]
fn table_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a b
.outputs c
.table a b -> c
11 1
.end
"#,
    )
    .unwrap();
}

#[test]
fn subckt_with_instance_name() {
    let ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs a b
.outputs z
.subckt adder|inst0 a=a b=b z=z
.end

.model adder
.inputs a b
.outputs z
.names a b z
11 1
.end
"#,
    )
    .unwrap();

    // Verify the instance name was stored
    assert_eq!(
        ast.entries,
        vec![
            BlifEntry::Model(Model {
                meta: ModelMeta {
                    name: "top".into(),
                    inputs: Some(vec!["a".into(), "b".into()]),
                    outputs: Some(vec!["z".into()]),
                    clocks: vec![]
                },
                commands: vec![
                    ModelCmdKind::SubModel {
                        name: "adder".into(),
                        map: vec![
                            ("a".into(), "a".into()),
                            ("b".into(), "b".into()),
                            ("z".into(), "z".into())
                        ],
                        instance_name: Some("inst0".into())
                    }
                    .into(),
                ],
                attr: Default::default(),
            }),
            BlifEntry::Model(Model {
                meta: ModelMeta {
                    name: "adder".into(),
                    inputs: Some(vec!["a".into(), "b".into()]),
                    outputs: Some(vec!["z".into()]),
                    clocks: vec![]
                },
                commands: vec![
                    ModelCmdKind::Gate(Gate {
                        meta: GateMeta {
                            inputs: vec!["a".into(), "b".into()],
                            output: "z".into(),
                            external_dc: false
                        },
                        lut: LUT(vec![(
                            [Tristate::True, Tristate::True].into_iter().collect(),
                            Some(true)
                        )])
                    })
                    .into(),
                ],
                attr: Default::default(),
            }),
        ]
    );
}

#[test]
fn input_required_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model flop
.inputs C CE D R S
.outputs Q
.attrib black box seq
.input_required C 0.0
.input_required CE 0.1
.input_required D 0.1
.input_required R 0.3
.input_required S 0.3
.end
"#,
    )
    .unwrap();
}

#[test]
fn output_arrival_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model flop
.inputs C CE D R S
.outputs Q
.attrib black box seq
.input_required C 0.0
.output_arrival Q 0.4
.end
"#,
    )
    .unwrap();
}

#[test]
fn spec_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.spec properties.ctl
.inputs a
.outputs z
.names a z
1 1
.end
"#,
    )
    .unwrap();
}

#[test]
fn gateinit_directive() {
    let _ast = parse_str_blif_to_ast(
        "top.blif",
        r#"
.model top
.inputs clk d
.outputs q
.latch d q re clk 0
.gateinit q=1
.end
"#,
    )
    .unwrap();
}

#[test]
fn mvsis_example_c880() {
    let source = include_str!("../blif-examples-from-mvsis/C880.blif");
    let ast = parse_str_blif_to_ast("C880.blif", source).unwrap();
    assert_eq!(ast.entries.len(), 1);
    let BlifEntry::Model(model) = &ast.entries[0];
    assert_eq!(model.meta.name, "C880.iscas");
    assert!(!model.meta.inputs.as_ref().unwrap().is_empty());
    assert!(!model.meta.outputs.as_ref().unwrap().is_empty());
    assert!(!model.commands.is_empty());
}

#[test]
fn mvsis_example_apex6() {
    let source = include_str!("../blif-examples-from-mvsis/apex6.blif");
    let ast = parse_str_blif_to_ast("apex6.blif", source).unwrap();
    assert_eq!(ast.entries.len(), 1);
}

#[test]
fn mvsis_example_frg2() {
    let source = include_str!("../blif-examples-from-mvsis/frg2.blif");
    let ast = parse_str_blif_to_ast("frg2.blif", source).unwrap();
    assert_eq!(ast.entries.len(), 1);
}

#[test]
fn mvsis_example_i9() {
    let source = include_str!("../blif-examples-from-mvsis/i9.blif");
    let ast = parse_str_blif_to_ast("i9.blif", source).unwrap();
    assert_eq!(ast.entries.len(), 1);
}

#[test]
fn mvsis_example_pj1() {
    let source = include_str!("../blif-examples-from-mvsis/pj1.blif");
    let ast = parse_str_blif_to_ast("pj1.blif", source).unwrap();
    assert_eq!(ast.entries.len(), 1);
    let BlifEntry::Model(model) = &ast.entries[0];
    assert_eq!(model.meta.name, "exCombCkt");
    assert!(model.commands.len() > 1000);
}

#[test]
fn mvsis_example_pj2() {
    let source = include_str!("../blif-examples-from-mvsis/pj2.blif");
    let ast = parse_str_blif_to_ast("pj2.blif", source).unwrap();
    assert_eq!(ast.entries.len(), 1);
    let BlifEntry::Model(model) = &ast.entries[0];
    assert_eq!(model.meta.name, "dcuCombCkt");
}

#[test]
fn mvsis_example_pj3() {
    let source = include_str!("../blif-examples-from-mvsis/pj3.blif");
    let ast = parse_str_blif_to_ast("pj3.blif", source).unwrap();
    assert_eq!(ast.entries.len(), 1);
    let BlifEntry::Model(model) = &ast.entries[0];
    assert_eq!(model.meta.name, "rcuCombCkt");
}

#[test]
fn mvsis_example_term1() {
    let source = include_str!("../blif-examples-from-mvsis/term1.blif");
    let ast = parse_str_blif_to_ast("term1.blif", source).unwrap();
    assert_eq!(ast.entries.len(), 1);
    let BlifEntry::Model(model) = &ast.entries[0];
    assert_eq!(model.meta.name, "term1");
    assert_eq!(model.meta.inputs.as_ref().unwrap().len(), 34);
    assert_eq!(model.meta.outputs.as_ref().unwrap().len(), 10);
}

#[test]
fn barbuf_alias_one_f() {
    // .barbuf (single 'f') is an ABC alias for .barbuff / .conn
    let ast = parse_str_blif_to_ast(
        "test.blif",
        r#"
.model top
.inputs a
.outputs b
.barbuf a b
.end
"#,
    )
    .unwrap();
    assert_eq!(
        ast.entries,
        vec![BlifEntry::Model(Model {
            meta: ModelMeta {
                name: "top".into(),
                inputs: Some(vec!["a".into()]),
                outputs: Some(vec!["b".into()]),
                clocks: vec![],
            },
            commands: vec![
                ModelCmdKind::Connect {
                    from: "a".into(),
                    to: "b".into(),
                }
                .into()
            ],
            attr: Default::default(),
        })]
    );
}

#[test]
fn delay_per_pair() {
    // .delay <in-sig> <out-sig> <delay> — ABC per-pair extension
    let ast = parse_str_blif_to_ast(
        "test.blif",
        r#"
.model top
.inputs a
.outputs z
.names a z
1 1
.delay a z 1.5
.end
"#,
    )
    .unwrap();
    assert_eq!(
        ast.entries,
        vec![BlifEntry::Model(Model {
            meta: ModelMeta {
                name: "top".into(),
                inputs: Some(vec!["a".into()]),
                outputs: Some(vec!["z".into()]),
                clocks: vec![],
            },
            commands: vec![
                ModelCmdKind::Gate(Gate {
                    meta: GateMeta {
                        inputs: vec!["a".into()],
                        output: "z".into(),
                        external_dc: false,
                    },
                    lut: LUT(vec![([Tristate::True].into_iter().collect(), Some(true))]),
                })
                .into(),
                ModelCmdKind::DelayConstraint(ModelDelayConstraint::DelayPerPair {
                    in_sig: "a".into(),
                    out_sig: "z".into(),
                    delay: 1.5,
                })
                .into(),
            ],
            attr: Default::default(),
        })]
    );
}
