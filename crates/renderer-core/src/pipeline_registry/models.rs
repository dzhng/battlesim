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
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0)
        .msaa(MSAA),
        render(
            "modelCaster",
            "modelCaster.wgsl",
            vec!["cameraLayout", "modelLayout"],
            Some(model_vertex()),
            None,
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0),
        render(
            "modelOpaque",
            "modelOpaque.wgsl",
            vec!["cameraLayout", "modelLayout", "fogLayout", "environment"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::GreaterEqual, false, 0)
        .msaa(MSAA),
        render(
            "modelPrepassCutout",
            "modelPrepassCutout.wgsl",
            vec!["cameraLayout", "modelLayout"],
            Some(model_vertex()),
            Some(fragment(None, vec![])),
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0)
        .msaa(MSAA),
        render(
            "modelCasterCutout",
            "modelCasterCutout.wgsl",
            vec!["cameraLayout", "modelLayout"],
            Some(model_vertex()),
            Some(fragment(None, vec![])),
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0),
        render(
            "modelCutout",
            "modelCutout.wgsl",
            vec!["cameraLayout", "modelLayout", "fogLayout", "environment"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::Equal, false, 0)
        .msaa(MSAA),
        render(
            "modelRooms",
            "modelRooms.wgsl",
            vec!["cameraLayout", "modelLayout", "fogLayout", "environment"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::GreaterEqual, false, 0)
        .msaa(MSAA),
        render(
            "modelBlended",
            "modelBlended.wgsl",
            vec!["cameraLayout", "modelLayout", "environment"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![
                    target_with(
                        HDR_FORMAT,
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
                    target_with(FOG_MASK_FORMAT, None, 0x0),
                ],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, false, 0)
        .msaa(MSAA),
        render(
            "modelGhost",
            "modelGhost.wgsl",
            vec!["cameraLayout", "modelLayout", "environment"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![target_with(
                    HDR_FORMAT,
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
            "modelXray",
            "modelXray.wgsl",
            vec!["cameraLayout", "modelLayout", "xrayReadLayout"],
            Some(model_vertex()),
            Some(fragment(
                None,
                vec![target_with(
                    OVERLAY_FORMAT,
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
        .depth(DEPTH_FORMAT, CompareFunction::Less, false, 65536)
        .msaa(MSAA),
        render(
            "modelXrayCount",
            "modelXrayCount.wgsl",
            vec!["cameraLayout", "modelLayout", "xrayCountLayout"],
            Some(model_vertex()),
            Some(fragment(None, vec![target_with(OVERLAY_FORMAT, None, 0x0)])),
        )
        .msaa(MSAA),
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
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0)
        .msaa(MSAA),
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
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0),
    ]
}
