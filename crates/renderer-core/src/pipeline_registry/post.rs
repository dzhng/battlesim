use super::*;
pub fn all() -> Vec<PipelineSpec> {
    vec![
        render(
            "bake",
            "bake.wgsl",
            vec!["skyBakeLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Rgba16Float)])),
        ),
        render(
            "background",
            "background.wgsl",
            vec!["skyBackgroundLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Rgba16Float)])),
        )
        .msaa(4),
        render(
            "pmremBake",
            "pmremBake.wgsl",
            vec!["sampled"],
            Some(vertex(
                None,
                vec![buffer(
                    24,
                    VertexStepMode::Vertex,
                    vec![
                        attr(VertexFormat::Float32x3, 0, 0),
                        attr(VertexFormat::Float32x2, 12, 1),
                        attr(VertexFormat::Float32, 20, 2),
                    ],
                )],
            )),
            Some(fragment(None, vec![target(TextureFormat::Rgba16Float)])),
        ),
        render(
            "pmremFilter",
            "pmremFilter.wgsl",
            vec!["filtering"],
            Some(vertex(
                None,
                vec![buffer(
                    24,
                    VertexStepMode::Vertex,
                    vec![
                        attr(VertexFormat::Float32x3, 0, 0),
                        attr(VertexFormat::Float32x2, 12, 1),
                        attr(VertexFormat::Float32, 20, 2),
                    ],
                )],
            )),
            Some(fragment(None, vec![target(TextureFormat::Rgba16Float)])),
        ),
        render(
            "backdropColour",
            "backdropColour.wgsl",
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
            "bloomHighpass",
            "bloomHighpass.wgsl",
            vec!["highpassLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Rgba16Float)])),
        ),
        render(
            "bloomBlur0",
            "bloomBlur0.wgsl",
            vec!["blurLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Rgba16Float)])),
        ),
        render(
            "bloomBlur1",
            "bloomBlur1.wgsl",
            vec!["blurLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Rgba16Float)])),
        ),
        render(
            "bloomBlur2",
            "bloomBlur2.wgsl",
            vec!["blurLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Rgba16Float)])),
        ),
        render(
            "bloomBlur3",
            "bloomBlur3.wgsl",
            vec!["blurLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Rgba16Float)])),
        ),
        render(
            "bloomBlur4",
            "bloomBlur4.wgsl",
            vec!["blurLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Rgba16Float)])),
        ),
        render(
            "bloomComposite",
            "bloomComposite.wgsl",
            vec!["compositeLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Rgba16Float)])),
        ),
        render(
            "postHighPlane",
            "postHighPlane.wgsl",
            vec!["finalLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Bgra8Unorm)])),
        ),
        render(
            "postUngraded",
            "postUngraded.wgsl",
            vec!["finalLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Bgra8Unorm)])),
        ),
        render(
            "postRaw",
            "postRaw.wgsl",
            vec!["finalLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Bgra8Unorm)])),
        ),
        render(
            "postLook",
            "postLook.wgsl",
            vec!["finalLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(TextureFormat::Bgra8Unorm)])),
        ),
    ]
}
