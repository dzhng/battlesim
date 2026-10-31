use super::*;
pub fn all() -> Vec<PipelineSpec> {
    vec![
        compute("fogTerrain", "fogTerrain.wgsl", vec!["buildLayout"], None),
        compute("fogMerge", "fogMerge.wgsl", vec!["buildLayout"], None),
        compute("fogCull", "fogCull.wgsl", vec!["cullLayout"], None),
        compute(
            "fogProbe",
            "fogProbe.wgsl",
            vec!["fogLayout", "probeLayout"],
            None,
        ),
        compute("fogShape", "fogShape.wgsl", vec!["shapeLayout"], None),
        compute(
            "fogWhole",
            "fogWhole.wgsl",
            vec!["wholeLayout", "fogLayout"],
            None,
        ),
        render(
            "fogMaskRows",
            "fogMaskRows.wgsl",
            vec!["rowsLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(FOG_DISTANCE_FORMAT)])),
        ),
        render(
            "fogMaskColumns",
            "fogMaskColumns.wgsl",
            vec!["columnsLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(FOG_DISTANCE_FORMAT)])),
        ),
        render(
            "fogMaskCompose",
            "fogMaskCompose.wgsl",
            vec!["composeLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(None, vec![target(HDR_FORMAT)])),
        )
        .msaa(MSAA),
        render(
            "fogMaskRim",
            "fogMaskRim.wgsl",
            vec!["composeLayout"],
            Some(vertex(None, vec![])),
            Some(fragment(
                None,
                vec![target_with(
                    DISPLAY_FORMAT,
                    Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
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
                )],
            )),
        ),
        render(
            "fogMaskRimHdr",
            "fogMaskRimHdr.wgsl",
            vec!["composeLayout"],
            Some(vertex(None, vec![])),
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
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    0xf,
                )],
            )),
        ),
    ]
}
