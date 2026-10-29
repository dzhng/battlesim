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
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        )
        .msaa(4),
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
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        ),
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
                    target_with(
                        TextureFormat::Rgba8Unorm,
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
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            false,
            0,
        )
        .msaa(4),
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
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        )
        .msaa(4),
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
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        ),
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
                vec![
                    target(TextureFormat::Rgba16Float),
                    target(TextureFormat::Rgba8Unorm),
                ],
            )),
        )
        .primitive(PrimitiveTopology::TriangleList, Some(Face::Back))
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::GreaterEqual,
            false,
            0,
        )
        .msaa(4),
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
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        )
        .msaa(4),
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
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        )
        .msaa(4),
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
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::Greater,
            true,
            0,
        ),
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
                vec![
                    target(TextureFormat::Rgba16Float),
                    target(TextureFormat::Rgba8Unorm),
                ],
            )),
        )
        .primitive(PrimitiveTopology::TriangleList, Some(Face::Back))
        .depth(
            TextureFormat::Depth32Float,
            CompareFunction::GreaterEqual,
            false,
            0,
        )
        .msaa(4),
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
        compute(
            "grassBuild",
            "grassBuild.wgsl",
            vec!["grassBuildLayout", "terrainLayout"],
            None,
        ),
        compute(
            "grassFinish",
            "grassFinish.wgsl",
            vec!["grassBuildLayout"],
            None,
        ),
    ]
}
