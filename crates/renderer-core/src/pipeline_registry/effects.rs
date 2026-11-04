use super::*;
pub fn all() -> Vec<PipelineSpec> {
    vec![
        render(
            "paintStill",
            "paintStill.wgsl",
            vec!["cameraLayout"],
            Some(vertex(
                None,
                vec![
                    buffer(
                        40,
                        VertexStepMode::Vertex,
                        vec![
                            attr(VertexFormat::Float32x3, 0, 0),
                            attr(VertexFormat::Float32x3, 12, 1),
                            attr(VertexFormat::Float32x4, 24, 2),
                        ],
                    ),
                    buffer(
                        32,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 16, 4),
                        ],
                    ),
                ],
            )),
            Some(fragment(
                None,
                vec![target_with(
                    OVERLAY_FORMAT,
                    Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    0xf,
                )],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, false, 0)
        .msaa(MSAA),
        render(
            "paintMarching",
            "paintMarching.wgsl",
            vec!["cameraLayout"],
            Some(vertex(
                None,
                vec![
                    buffer(
                        40,
                        VertexStepMode::Vertex,
                        vec![
                            attr(VertexFormat::Float32x3, 0, 0),
                            attr(VertexFormat::Float32x3, 12, 1),
                            attr(VertexFormat::Float32x4, 24, 2),
                        ],
                    ),
                    buffer(
                        32,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 16, 4),
                        ],
                    ),
                ],
            )),
            Some(fragment(
                None,
                vec![target_with(
                    OVERLAY_FORMAT,
                    Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    0xf,
                )],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, false, 0)
        .msaa(MSAA),
        render(
            "overlayOpaque",
            "overlayOpaque.wgsl",
            vec!["cameraLayout"],
            Some(vertex(
                None,
                vec![
                    buffer(
                        40,
                        VertexStepMode::Vertex,
                        vec![
                            attr(VertexFormat::Float32x3, 0, 0),
                            attr(VertexFormat::Float32x3, 12, 1),
                            attr(VertexFormat::Float32x4, 24, 2),
                        ],
                    ),
                    buffer(
                        32,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 16, 4),
                        ],
                    ),
                ],
            )),
            Some(fragment(
                None,
                vec![target_with(
                    OVERLAY_FORMAT,
                    Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    0xf,
                )],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0)
        .msaa(MSAA),
        render(
            "overlayTranslucent",
            "overlayTranslucent.wgsl",
            vec!["cameraLayout"],
            Some(vertex(
                None,
                vec![
                    buffer(
                        40,
                        VertexStepMode::Vertex,
                        vec![
                            attr(VertexFormat::Float32x3, 0, 0),
                            attr(VertexFormat::Float32x3, 12, 1),
                            attr(VertexFormat::Float32x4, 24, 2),
                        ],
                    ),
                    buffer(
                        32,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 16, 4),
                        ],
                    ),
                ],
            )),
            Some(fragment(
                None,
                vec![target_with(
                    OVERLAY_FORMAT,
                    Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    0xf,
                )],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, false, 0)
        .msaa(MSAA),
        render(
            "overlayComposite",
            "overlayComposite.wgsl",
            vec!["overlaySource"],
            Some(vertex(None, vec![])),
            Some(fragment(
                None,
                vec![target_with(
                    DISPLAY_FORMAT,
                    Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    0xf,
                )],
            )),
        ),
        render(
            "glowRows",
            "glowRows.wgsl",
            vec!["glowRowsLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(OVERLAY_FORMAT)])),
        ),
        render(
            "glowColumns",
            "glowColumns.wgsl",
            vec!["glowColumnsLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(OVERLAY_FORMAT)])),
        ),
        render(
            "effects",
            "effects.wgsl",
            vec!["effects"],
            Some(vertex(
                Some("spriteVertex"),
                vec![buffer(
                    64,
                    VertexStepMode::Instance,
                    vec![
                        attr(VertexFormat::Float32x4, 0, 0),
                        attr(VertexFormat::Float32x4, 16, 1),
                        attr(VertexFormat::Float32x4, 32, 2),
                        attr(VertexFormat::Float32x4, 48, 3),
                    ],
                )],
            )),
            Some(fragment(
                Some("spriteFragment"),
                vec![
                    target_with(
                        HDR_FORMAT,
                        Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::One,
                                dst_factor: BlendFactor::OneMinusSrcAlpha,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent {
                                src_factor: BlendFactor::Zero,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                        }),
                        0xf,
                    ),
                    target_with(
                        FOG_MASK_FORMAT,
                        Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::Zero,
                                dst_factor: BlendFactor::OneMinusSrcAlpha,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent {
                                src_factor: BlendFactor::Zero,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                        }),
                        0x3,
                    ),
                ],
            )),
        ),
    ]
}
