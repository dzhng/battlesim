// What the wave field does with each frame's waves (`Presentation::waves`):
// a new epoch or a battle gone clears it, even in a frame with no step, so
// one side's waves never linger for the next; and every texel the camera
// moves reaches the field, whether or not the frame steps it.
use super::*;
use presentation::wave::WaveStep;

fn frame(steps: u32, epoch: u32, shift: [i32; 2], splats: Vec<WaveSplat>) -> WaveFrame {
    WaveFrame {
        step: WaveStep {
            steps,
            epoch,
            origin_texel: [0, 0],
            shift,
        },
        splats,
    }
}

#[test]
fn a_new_epoch_clears_the_field_even_in_a_frame_with_no_step() {
    let plan = wave_plan(Some(&frame(0, 8, [0, 0], vec![])), Some(7));
    assert!(plan.clear);
    assert_eq!(plan.epoch, Some(8));
}

#[test]
fn a_battle_gone_clears_the_field_once() {
    let gone = wave_plan(None, Some(7));
    assert!(gone.clear);
    assert!(gone.runs.is_empty());
    assert!(!wave_plan(None, gone.epoch).clear);
}

#[test]
fn every_texel_the_camera_moves_reaches_the_field_stepped_or_not() {
    let mut epoch = Some(3);
    let mut moved = 0;
    // Panning 8 texels a frame at twice the step rate: a step every other frame.
    for steps in [0, 1, 0, 1, 0, 0] {
        let plan = wave_plan(Some(&frame(steps, 3, [8, 0], vec![])), epoch);
        epoch = plan.epoch;
        assert!(!plan.clear);
        moved += plan.runs.iter().map(|r| r.shift[0]).sum::<i32>();
    }
    assert_eq!(moved, 48);
}

#[test]
fn only_a_frames_first_step_carries_its_shift_and_a_still_run_never_steps() {
    let plan = wave_plan(Some(&frame(3, 3, [5, -2], vec![])), Some(3));
    let runs: Vec<_> = plan.runs.iter().map(|r| (r.shift, r.still)).collect();
    assert_eq!(runs, [([5, -2], 0), ([0, 0], 0), ([0, 0], 0)]);
    let still = wave_plan(Some(&frame(0, 3, [5, -2], vec![])), Some(3));
    let runs: Vec<_> = still
        .runs
        .iter()
        .map(|r| (r.shift, r.step, r.still))
        .collect();
    assert_eq!(runs, [([5, -2], 0, 1)]);
}

#[test]
fn each_step_keeps_its_own_splats_however_the_frames_batch_the_steps() {
    // `n` splats at step `step`, each marked by its index.
    let splats = |step: u32, n: usize| -> Vec<WaveSplat> {
        (0..n)
            .map(|i| WaveSplat {
                splat: [i as f32, 0.0, 0.0, 0.0],
                step: step as f32,
                ..Default::default()
            })
            .collect()
    };
    // Two steps of 300 splats each, over the budget: in one frame (steps 0
    // and 1), or in two frames of one step each.
    let together = [splats(0, 300), splats(1, 300)].concat();
    let together = wave_plan(Some(&frame(2, 1, [0, 0], together)), Some(1));
    let first = wave_plan(Some(&frame(1, 1, [0, 0], splats(0, 300))), Some(1));
    let second = wave_plan(Some(&frame(1, 1, [0, 0], splats(0, 300))), Some(1));
    assert_eq!(
        together.splats.len(),
        first.splats.len() + second.splats.len()
    );
    assert_eq!(first.splats.len(), STEP_SPLATS);
}

#[test]
fn splat_binding_matches_allocated_bytes() {
    let source = crate::pipelines::builtin_source("waveStep.wgsl").unwrap();
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    let (_, splats) = module
        .global_variables
        .iter()
        .find(|(_, global)| global.name.as_deref() == Some("splats"))
        .unwrap();
    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).unwrap();
    assert_eq!(
        layouter[splats.ty].size as usize, SPLATS_BYTES,
        "splats binding versus renderer allocation"
    );
}
