use super::*;

#[test]
fn canonical_preparation_republishes_the_current_ui_frame() {
    let mut presentation = Presentation::new(None, None).unwrap();
    let camera = [
        12.0, -20.0, 30.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.1, 320.0, 180.0,
    ];
    presentation
        .frame(1.0, &camera, &[], false, None, "", "", 0)
        .unwrap();
    assert_eq!(presentation.frame_header.clock, 1.0);

    presentation
        .core_mut()
        .frame(9.0, &camera_of(&camera), &Ui::default(), 0)
        .unwrap();
    presentation.refresh_frame_header();

    assert_eq!(presentation.frame_header.clock, 9.0);
    assert_eq!(presentation.frame_header.index, 2);
    assert_eq!(
        presentation.frame_header.palettes,
        set(&presentation.core().last_frame().palettes)
    );
}
