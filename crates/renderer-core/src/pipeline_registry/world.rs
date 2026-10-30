use super::*;
pub fn all() -> Vec<PipelineSpec> {
    vec![
        render(
            "worldPrepass",
            "worldPrepass.wgsl",
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
            None,
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0)
        .msaa(MSAA),
        render(
            "worldCaster",
            "worldCaster.wgsl",
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
            None,
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0),
        render(
            "worldOpaque",
            "worldOpaque.wgsl",
            vec!["cameraLayout", "environment", "fogLayout"],
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
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::GreaterEqual, false, 0)
        .msaa(MSAA),
        render(
            "worldWater",
            "worldWater.wgsl",
            vec!["cameraLayout", "terrainLayout", "fogLayout", "environment"],
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
                    target_with(
                        FOG_MASK_FORMAT,
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
                ],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, false, 0)
        .msaa(MSAA),
        render(
            "worldTerrain",
            "worldTerrain.wgsl",
            vec!["cameraLayout", "terrainLayout", "fogLayout", "environment"],
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
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::GreaterEqual, false, 0)
        .msaa(MSAA),
        render(
            "worldBackdrop",
            "worldBackdrop.wgsl",
            vec!["cameraLayout", "terrainLayout", "fogLayout", "environment"],
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
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::GreaterEqual, false, 0)
        .msaa(MSAA),
        render(
            "sceneryPrepass",
            "sceneryPrepass.wgsl",
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
                        64,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 48, 4),
                            attr(VertexFormat::Float32x4, 32, 5),
                        ],
                    ),
                ],
            )),
            None,
        )
        .primitive(PrimitiveTopology::TriangleList, Some(Face::Back))
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0)
        .msaa(MSAA),
        render(
            "sceneryCaster",
            "sceneryCaster.wgsl",
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
                        64,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 48, 4),
                            attr(VertexFormat::Float32x4, 32, 5),
                        ],
                    ),
                ],
            )),
            None,
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0),
        render(
            "forestColour",
            "forestColour.wgsl",
            vec!["cameraLayout", "fogLayout", "environment"],
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
                        64,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 48, 4),
                            attr(VertexFormat::Float32x4, 32, 5),
                        ],
                    ),
                ],
            )),
            Some(fragment(
                None,
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .primitive(PrimitiveTopology::TriangleList, Some(Face::Back))
        .depth(DEPTH_FORMAT, CompareFunction::GreaterEqual, false, 0)
        .msaa(MSAA),
        render(
            "dressingPrepass",
            "dressingPrepass.wgsl",
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
                        64,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 48, 4),
                            attr(VertexFormat::Float32x4, 32, 5),
                        ],
                    ),
                ],
            )),
            None,
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0)
        .msaa(MSAA),
        render(
            "dressingColour",
            "dressingColour.wgsl",
            vec!["cameraLayout", "fogLayout", "environment"],
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
                        64,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 48, 4),
                            attr(VertexFormat::Float32x4, 32, 5),
                        ],
                    ),
                ],
            )),
            Some(fragment(
                None,
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::GreaterEqual, false, 0)
        .msaa(MSAA),
        render(
            "felledPrepass",
            "felledPrepass.wgsl",
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
                        128,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 48, 4),
                            attr(VertexFormat::Float32x4, 32, 5),
                            attr(VertexFormat::Float32x4, 16, 6),
                            attr(VertexFormat::Float32x4, 64, 7),
                            attr(VertexFormat::Float32x4, 80, 8),
                        ],
                    ),
                ],
            )),
            None,
        )
        .primitive(PrimitiveTopology::TriangleList, Some(Face::Back))
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0)
        .msaa(MSAA),
        render(
            "felledCaster",
            "felledCaster.wgsl",
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
                        128,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 48, 4),
                            attr(VertexFormat::Float32x4, 32, 5),
                            attr(VertexFormat::Float32x4, 16, 6),
                            attr(VertexFormat::Float32x4, 64, 7),
                            attr(VertexFormat::Float32x4, 80, 8),
                        ],
                    ),
                ],
            )),
            None,
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0),
        render(
            "felledColour",
            "felledColour.wgsl",
            vec!["cameraLayout", "fogLayout", "environment"],
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
                        128,
                        VertexStepMode::Instance,
                        vec![
                            attr(VertexFormat::Float32x4, 0, 3),
                            attr(VertexFormat::Float32x4, 48, 4),
                            attr(VertexFormat::Float32x4, 32, 5),
                            attr(VertexFormat::Float32x4, 16, 6),
                            attr(VertexFormat::Float32x4, 64, 7),
                            attr(VertexFormat::Float32x4, 80, 8),
                        ],
                    ),
                ],
            )),
            Some(fragment(
                None,
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .primitive(PrimitiveTopology::TriangleList, Some(Face::Back))
        .depth(DEPTH_FORMAT, CompareFunction::GreaterEqual, false, 0)
        .msaa(MSAA),
        render(
            "grassDraw",
            "grassDraw.wgsl",
            vec![
                "cameraLayout",
                "grassDrawLayout",
                "fogLayout",
                "environment",
            ],
            Some(vertex(None, vec![])),
            Some(fragment(
                None,
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .depth(DEPTH_FORMAT, CompareFunction::Greater, true, 0)
        .msaa(MSAA),
        compute(
            "grassCount",
            "grassBuild.wgsl",
            vec!["grassBuildLayout", "terrainLayout"],
            Some("countFn"),
        ),
        compute(
            "grassBuild",
            "grassBuild.wgsl",
            vec!["grassBuildLayout", "terrainLayout"],
            Some("buildFn"),
        ),
        compute(
            "grassFinish",
            "grassFinish.wgsl",
            vec!["grassBuildLayout"],
            None,
        ),
    ]
}
