use super::*;
/// The unit info cards' screen-space primitives (`crate::readouts`): one
/// instanced quad each, premultiplied into the readout layer.
pub fn all() -> Vec<PipelineSpec> {
    let premultiplied = BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    };
    vec![render(
        "readoutQuads",
        "readoutQuads.wgsl",
        vec!["sampled"],
        Some(vertex(
            None,
            vec![buffer(
                144,
                VertexStepMode::Instance,
                (0..9)
                    .map(|i| attr(VertexFormat::Float32x4, u64::from(i) * 16, i))
                    .collect(),
            )],
        )),
        Some(fragment(
            None,
            vec![target_with(
                OVERLAY_FORMAT,
                Some(BlendState {
                    color: premultiplied,
                    alpha: premultiplied,
                }),
                0xf,
            )],
        )),
    )]
}
