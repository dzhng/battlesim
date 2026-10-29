use super::*;
pub fn all() -> Vec<PipelineSpec> {
    vec![
        render(
            "modelPrepass",
            "modelPrepass.wgsl",
            vec!["cameraLayout", "modelLayout"],
            Some(model_vertex()),
            None,
        )
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        )
        .msaa(4),
        render(
            "modelCaster",
            "modelCaster.wgsl",
            vec!["cameraLayout", "modelLayout"],
            Some(model_vertex()),
            None,
        )
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        ),
        render(
            "modelOpaque",
            "modelOpaque.wgsl",
            vec!["cameraLayout", "modelLayout", "fogLayout", "environment"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![
                    target(TextureFormat::Rgba16Float),
                    target(TextureFormat::Rgba8Unorm),
                ],
            )),
        )
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::GreaterEqual,
            false,
            0,
        )
        .msaa(4),
        render(
            "modelPrepassCutout",
            "modelPrepassCutout.wgsl",
            vec!["cameraLayout", "modelLayout"],
            Some(model_vertex()),
            Some(fragment(None, vec![])),
        )
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        )
        .msaa(4),
        render(
            "modelCasterCutout",
            "modelCasterCutout.wgsl",
            vec!["cameraLayout", "modelLayout"],
            Some(model_vertex()),
            Some(fragment(None, vec![])),
        )
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        ),
        render(
            "modelCutout",
            "modelCutout.wgsl",
            vec!["cameraLayout", "modelLayout", "fogLayout", "environment"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![
                    target(TextureFormat::Rgba16Float),
                    target(TextureFormat::Rgba8Unorm),
                ],
            )),
        )
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Equal,
            false,
            0,
        )
        .msaa(4),
        render(
            "modelRooms",
            "modelRooms.wgsl",
            vec!["cameraLayout", "modelLayout", "fogLayout", "environment"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![
                    target(TextureFormat::Rgba16Float),
                    target(TextureFormat::Rgba8Unorm),
                ],
            )),
        )
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::GreaterEqual,
            false,
            0,
        )
        .msaa(4),
        render(
            "modelBlended",
            "modelBlended.wgsl",
            vec!["cameraLayout", "modelLayout", "environment"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![
                    target_with(
                        TextureFormat::Rgba16Float,
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
                    ),
                    target_with(TextureFormat::Rgba8Unorm, None, 0x0),
                ],
            )),
        )
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            false,
            0,
        )
        .msaa(4),
        render(
            "modelGhost",
            "modelGhost.wgsl",
            vec!["cameraLayout", "modelLayout", "environment"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![target_with(
                    TextureFormat::Rgba16Float,
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
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            false,
            0,
        )
        .msaa(4),
        render(
            "modelXray",
            "modelXray.wgsl",
            vec!["cameraLayout", "modelLayout", "xrayReadLayout"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![target_with(
                    TextureFormat::Rgba8Unorm,
                    Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Max,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Max,
                        },
                    }),
                    0xf,
                )],
            )),
        )
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Less,
            false,
            65536,
        )
        .msaa(4),
        render(
            "modelXrayCount",
            "modelXrayCount.wgsl",
            vec!["cameraLayout", "modelLayout", "xrayCountLayout"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![target_with(TextureFormat::Rgba8Unorm, None, 0x0)],
            )),
        )
        .msaa(4),
        render(
            "modelCards",
            "modelCards.wgsl",
            vec!["cameraLayout", "cardLayout", "environment", "fogLayout"],
            Some(vertex(
                None,
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
                None,
                vec![
                    target(TextureFormat::Rgba16Float),
                    target(TextureFormat::Rgba8Unorm),
                ],
            )),
        )
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        )
        .msaa(4),
        render(
            "modelImpostor",
            "modelImpostor.wgsl",
            vec!["cameraLayout", "modelLayout"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![
                    target(TextureFormat::Rgba8Unorm),
                    target(TextureFormat::Rgba8Unorm),
                    target(TextureFormat::Rgba8Unorm),
                ],
            )),
        )
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        ),
    ]
}
