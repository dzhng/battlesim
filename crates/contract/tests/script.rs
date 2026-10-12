use contract::ids::Side;
use contract::observation::ObservationFrame;
use contract::script::{CompiledScript, RuntimeErrorKind, ScriptInput, ScriptLimits};

fn input<'a>(frame: &'a ObservationFrame) -> ScriptInput<'a> {
    ScriptInput::new(Side::Blue, frame, 0xfeed_beef)
}

#[test]
fn deterministic_script_runs_from_observation_and_seed() {
    let frame = ObservationFrame {
        tick: 7,
        ..ObservationFrame::default()
    };
    let script = CompiledScript::compile(
        r#"
        let draws = random(100);
        repeat (2) { event("pulse"); }
        if tick() == 7 && true { victory(true); }
        "#,
    )
    .unwrap();
    let first = script.run(&input(&frame)).unwrap();
    let second = script.run(&input(&frame)).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first
            .events
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        ["pulse", "pulse"]
    );
    assert_eq!(first.victory, Some(true));
    assert_eq!(first.instructions, second.instructions);
}

#[test]
fn order_output_is_ordinary_command_vocabulary() {
    let frame = ObservationFrame::default();
    let script = CompiledScript::compile(
        r#"order move(3, 12.5, 4.0); order engagement(3, "return_fire_only"); order stop(3);"#,
    )
    .unwrap();
    let output = script.run(&input(&frame)).unwrap();
    assert_eq!(output.commands.len(), 3);
    assert_eq!(output.commands[0].side, Side::Blue);
    assert!(matches!(
        output.commands[0].order,
        contract::command::Order::Move { .. }
    ));
    assert!(matches!(
        output.commands[1].order,
        contract::command::Order::SetEngagement { .. }
    ));
    assert!(matches!(
        output.commands[2].order,
        contract::command::Order::Stop { .. }
    ));
}

#[test]
fn hidden_state_and_ambient_constructs_are_refused_with_source_spans() {
    let error = CompiledScript::compile("if battle.secret { event(\"x\"); }").unwrap_err();
    assert_eq!(error.diagnostics[0].code, "character");
    assert!(error.diagnostics[0].span.line >= 1);
    let error = CompiledScript::compile("while true { event(\"x\"); }").unwrap_err();
    assert_eq!(error.diagnostics[0].code, "statement");
}

#[test]
fn fuel_and_output_limits_are_deterministic_refusals() {
    let frame = ObservationFrame::default();
    let script = CompiledScript::compile("repeat (100) { event(\"x\"); }").unwrap();
    let error = script
        .run_with_limits(
            &input(&frame),
            ScriptLimits {
                max_instructions: 10_000,
                max_loop_iterations: 100,
                max_output_events: 3,
                ..ScriptLimits::default()
            },
        )
        .unwrap_err();
    assert_eq!(error.kind, RuntimeErrorKind::OutputLimit);

    let script = CompiledScript::compile("repeat (100) { event(\"x\"); }").unwrap();
    let error = script
        .run_with_limits(
            &input(&frame),
            ScriptLimits {
                max_instructions: 4,
                ..ScriptLimits::default()
            },
        )
        .unwrap_err();
    assert_eq!(error.kind, RuntimeErrorKind::FuelExhausted);
}

#[test]
fn identity_includes_runtime_version_and_source_hash() {
    let a = CompiledScript::compile("victory(true);").unwrap();
    let b = CompiledScript::compile("victory(false);").unwrap();
    assert_eq!(a.runtime_version(), "scenario-script-1");
    assert_ne!(a.source_hash(), b.source_hash());
    assert_ne!(a.identity(), b.identity());
}

#[test]
fn return_condition_is_a_portable_victory_script() {
    let script = CompiledScript::compile("return tick() == 0").unwrap();
    let frame = ObservationFrame::default();
    assert_eq!(script.run(&input(&frame)).unwrap().victory, Some(true));
}

#[test]
fn neutral_script_can_observe_but_cannot_emit_commands() {
    let script = CompiledScript::compile("order stop(0);").unwrap();
    let frame = ObservationFrame::default();
    let error = script
        .run(&ScriptInput::new(Side::Neutral, &frame, 17))
        .unwrap_err();
    assert_eq!(error.kind, RuntimeErrorKind::InvalidOrder);
    assert!(error.message.contains("neutral"));
}
