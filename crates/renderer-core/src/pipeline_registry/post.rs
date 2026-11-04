use super::*;
pub fn all() -> Vec<PipelineSpec> {
    vec![
        render(
            "bake",
            "bake.wgsl",
            vec!["skyBakeLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(HDR_FORMAT)])),
        ),
        render(
            "background",
            "background.wgsl",
            vec!["skyBackgroundLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(HDR_FORMAT)])),
        )
        .msaa(MSAA),
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
            Some(fragment(None, vec![target(HDR_FORMAT)])),
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
            Some(fragment(None, vec![target(HDR_FORMAT)])),
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
                vec![target(HDR_FORMAT), target(FOG_MASK_FORMAT)],
            )),
        )
        .primitive(PrimitiveTopology::TriangleList, Some(Face::Back))
        .depth(DEPTH_FORMAT, CompareFunction::GreaterEqual, false, 0)
        .msaa(MSAA),
        render(
            "bloomHighpass",
            "bloomHighpass.wgsl",
            vec!["highpassLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(HDR_FORMAT)])),
        ),
        render(
            "bloomBlur0",
            "bloomBlur0.wgsl",
            vec!["blurLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(HDR_FORMAT)])),
        ),
        render(
            "bloomBlur1",
            "bloomBlur1.wgsl",
            vec!["blurLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(HDR_FORMAT)])),
        ),
        render(
            "bloomBlur2",
            "bloomBlur2.wgsl",
            vec!["blurLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(HDR_FORMAT)])),
        ),
        render(
            "bloomBlur3",
            "bloomBlur3.wgsl",
            vec!["blurLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(HDR_FORMAT)])),
        ),
        render(
            "bloomBlur4",
            "bloomBlur4.wgsl",
            vec!["blurLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(HDR_FORMAT)])),
        ),
        render(
            "bloomComposite",
            "bloomComposite.wgsl",
            vec!["compositeLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(HDR_FORMAT)])),
        ),
        render(
            "postHighPlane",
            "postHighPlane.wgsl",
            vec!["finalLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(DISPLAY_FORMAT)])),
        ),
        render(
            "postUngraded",
            "postUngraded.wgsl",
            vec!["finalLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(DISPLAY_FORMAT)])),
        ),
        render(
            "postRaw",
            "postRaw.wgsl",
            vec!["finalLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(DISPLAY_FORMAT)])),
        ),
        render(
            "postLook",
            "postLook.wgsl",
            vec!["finalLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(DISPLAY_FORMAT)])),
        ),
    ]
}
