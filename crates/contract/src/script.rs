//! Deterministic authored scenario scripts.
//!
//! Scripts are compiled by this crate into a small, closed interpreter.  The
//! interpreter has no access to a `Battle`, the clock, a filesystem, or host
//! functions other than the observation projection and seeded random stream
//! exposed by [`ScriptInput`].  Output is ordinary command vocabulary so the
//! simulation remains the only authority that applies orders.
use crate::command::{Engagement, MoveDirection, Order, RoutePolicy, TargetRef};
use crate::identity::bytes_hash;
use crate::ids::{Side, Tick, UnitId};
use crate::observation::{ObservationFrame, ObservedTargetId};
use crate::random::mix;
use serde::{Deserialize, Serialize};

pub const SCRIPT_RUNTIME_VERSION: &str = "scenario-script-1";
pub const DEFAULT_MAX_SOURCE_BYTES: usize = 256 * 1024;
const MAX_PARSE_NESTING: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptLimits {
    pub max_instructions: u32,
    pub max_loop_iterations: u32,
    pub max_output_commands: usize,
    pub max_output_events: usize,
    pub max_variables: usize,
}

impl Default for ScriptLimits {
    fn default() -> Self {
        Self {
            max_instructions: 20_000,
            max_loop_iterations: 256,
            max_output_commands: 128,
            max_output_events: 128,
            max_variables: 128,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptDiagnostic {
    pub code: &'static str,
    pub span: SourceSpan,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptCompileError {
    pub diagnostics: Vec<ScriptDiagnostic>,
}

impl std::fmt::Display for ScriptCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, d) in self.diagnostics.iter().enumerate() {
            if i > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{}:{}: {}", d.span.line, d.span.column, d.message)?;
        }
        Ok(())
    }
}
impl std::error::Error for ScriptCompileError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeErrorKind {
    FuelExhausted,
    LoopLimit,
    OutputLimit,
    VariableLimit,
    TypeError,
    InvalidOrder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptRuntimeError {
    pub kind: RuntimeErrorKind,
    pub span: Option<SourceSpan>,
    pub instructions: u32,
    pub message: String,
}

impl std::fmt::Display for ScriptRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ScriptRuntimeError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptError {
    Compile(ScriptCompileError),
    Runtime(ScriptRuntimeError),
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compile(e) => e.fmt(f),
            Self::Runtime(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for ScriptError {}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScriptOutput {
    pub commands: Vec<ScriptCommand>,
    pub events: Vec<ScriptEvent>,
    pub victory: Option<bool>,
    pub instructions: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScriptCommand {
    pub side: Side,
    pub slot: Option<String>,
    pub order: Order,
    pub queued: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptEvent {
    pub side: Side,
    pub name: String,
}

/// The only state supplied to a script.  The full observation is borrowed by
/// the host, but scripts can read only the scalar projection below.
pub struct ScriptInput<'a> {
    pub side: Side,
    pub slot: Option<String>,
    pub tick: Tick,
    pub observation: &'a ObservationFrame,
    pub rng_seed: u64,
}

impl<'a> ScriptInput<'a> {
    pub fn new(side: Side, observation: &'a ObservationFrame, rng_seed: u64) -> Self {
        Self {
            side,
            slot: None,
            tick: observation.tick,
            observation,
            rng_seed,
        }
    }

    pub fn with_slot(mut self, slot: impl Into<String>) -> Self {
        self.slot = Some(slot.into());
        self
    }
}

#[derive(Clone, Debug)]
pub struct CompiledScript {
    source_hash: String,
    source: String,
    program: Vec<Stmt>,
}

impl CompiledScript {
    pub fn compile(source: &str) -> Result<Self, ScriptCompileError> {
        if source.trim().is_empty() {
            return Err(ScriptCompileError {
                diagnostics: vec![diag(
                    "empty_script",
                    SourceSpan {
                        start: 0,
                        end: source.len(),
                        line: 1,
                        column: 1,
                    },
                    "script source must contain at least one statement",
                )],
            });
        }
        if source.len() > DEFAULT_MAX_SOURCE_BYTES {
            return Err(ScriptCompileError {
                diagnostics: vec![diag(
                    "size_limit",
                    SourceSpan {
                        start: 0,
                        end: source.len(),
                        line: 1,
                        column: 1,
                    },
                    "script source exceeds the 256 KiB limit",
                )],
            });
        }
        let tokens = Lexer::new(source).lex().map_err(|d| ScriptCompileError {
            diagnostics: vec![d],
        })?;
        let mut parser = Parser::new(tokens);
        let program = parser
            .program()
            .map_err(|diagnostics| ScriptCompileError { diagnostics })?;
        Ok(Self {
            source_hash: bytes_hash(source.as_bytes()),
            source: source.to_owned(),
            program,
        })
    }

    pub fn runtime_version(&self) -> &'static str {
        SCRIPT_RUNTIME_VERSION
    }

    pub fn source_hash(&self) -> &str {
        &self.source_hash
    }

    /// Identity includes the interpreter version and source hash, so changing
    /// semantics cannot silently reuse a replay or scenario identity.
    pub fn identity(&self) -> String {
        bytes_hash(format!("{SCRIPT_RUNTIME_VERSION}:{}", self.source_hash).as_bytes())
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn run(&self, input: &ScriptInput<'_>) -> Result<ScriptOutput, ScriptRuntimeError> {
        self.run_with_limits(input, ScriptLimits::default())
    }

    pub fn run_with_limits(
        &self,
        input: &ScriptInput<'_>,
        limits: ScriptLimits,
    ) -> Result<ScriptOutput, ScriptRuntimeError> {
        let mut vm = Vm {
            input,
            limits,
            instructions: 0,
            loop_iterations: 0,
            rng_counter: 0,
            variables: Vec::new(),
            commands: Vec::new(),
            events: Vec::new(),
            victory: None,
        };
        vm.run_program(&self.program)?;
        Ok(ScriptOutput {
            commands: vm.commands,
            events: vm.events,
            victory: vm.victory,
            instructions: vm.instructions,
        })
    }
}

#[derive(Clone, Debug)]
enum Stmt {
    Let {
        name: String,
        value: Expr,
        span: SourceSpan,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
        span: SourceSpan,
    },
    Repeat {
        count: Expr,
        body: Vec<Stmt>,
        span: SourceSpan,
    },
    Order {
        name: String,
        args: Vec<Expr>,
        span: SourceSpan,
    },
    Event {
        value: Expr,
        span: SourceSpan,
    },
    Victory {
        value: Expr,
        span: SourceSpan,
    },
}

#[derive(Clone, Debug)]
enum Expr {
    Literal(Value, SourceSpan),
    Variable(String, SourceSpan),
    Call {
        name: String,
        args: Vec<Expr>,
        span: SourceSpan,
    },
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
        span: SourceSpan,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: SourceSpan,
    },
}

#[derive(Clone, Copy, Debug)]
enum UnaryOp {
    Neg,
    Not,
}
#[derive(Clone, Copy, Debug)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Number(f64),
    Bool(bool),
    Text(String),
}

impl Value {
    fn bool(&self, span: SourceSpan) -> Result<bool, ScriptRuntimeError> {
        match self {
            Self::Bool(v) => Ok(*v),
            _ => Err(runtime(
                RuntimeErrorKind::TypeError,
                span,
                0,
                "expected boolean",
            )),
        }
    }
    fn number(&self, span: SourceSpan) -> Result<f64, ScriptRuntimeError> {
        match self {
            Self::Number(v) if v.is_finite() => Ok(*v),
            _ => Err(runtime(
                RuntimeErrorKind::TypeError,
                span,
                0,
                "expected finite number",
            )),
        }
    }
    fn text(&self, span: SourceSpan) -> Result<String, ScriptRuntimeError> {
        match self {
            Self::Text(v) => Ok(v.clone()),
            _ => Err(runtime(
                RuntimeErrorKind::TypeError,
                span,
                0,
                "expected string",
            )),
        }
    }
}

struct Vm<'a> {
    input: &'a ScriptInput<'a>,
    limits: ScriptLimits,
    instructions: u32,
    loop_iterations: u32,
    rng_counter: u64,
    variables: Vec<(String, Value)>,
    commands: Vec<ScriptCommand>,
    events: Vec<ScriptEvent>,
    victory: Option<bool>,
}

impl<'a> Vm<'a> {
    fn step(&mut self, span: SourceSpan) -> Result<(), ScriptRuntimeError> {
        self.instructions = self.instructions.saturating_add(1);
        if self.instructions > self.limits.max_instructions {
            return Err(runtime(
                RuntimeErrorKind::FuelExhausted,
                span,
                self.instructions,
                "script instruction budget exhausted",
            ));
        }
        Ok(())
    }
    fn run_program(&mut self, body: &[Stmt]) -> Result<(), ScriptRuntimeError> {
        for stmt in body {
            self.run_stmt(stmt)?;
        }
        Ok(())
    }
    fn run_stmt(&mut self, stmt: &Stmt) -> Result<(), ScriptRuntimeError> {
        let span = stmt_span(stmt);
        self.step(span)?;
        match stmt {
            Stmt::Let { name, value, .. } => {
                let v = self.eval(value)?;
                if self.variables.iter().all(|(n, _)| n != name) {
                    if self.variables.len() >= self.limits.max_variables {
                        return Err(runtime(
                            RuntimeErrorKind::VariableLimit,
                            span,
                            self.instructions,
                            "script variable limit exhausted",
                        ));
                    }
                    self.variables.push((name.clone(), v));
                } else if let Some((_, old)) = self.variables.iter_mut().find(|(n, _)| n == name) {
                    *old = v;
                }
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                if self.eval(condition)?.bool(span)? {
                    self.run_program(then_body)?;
                } else {
                    self.run_program(else_body)?;
                }
            }
            Stmt::Repeat { count, body, .. } => {
                let count = self.eval(count)?.number(span)?;
                if !count.is_finite() || count < 0.0 || count.fract() != 0.0 {
                    return Err(runtime(
                        RuntimeErrorKind::TypeError,
                        span,
                        self.instructions,
                        "repeat count must be a non-negative integer",
                    ));
                }
                let count = count as u32;
                if count > self.limits.max_loop_iterations {
                    return Err(runtime(
                        RuntimeErrorKind::LoopLimit,
                        span,
                        self.instructions,
                        "repeat count exceeds loop limit",
                    ));
                }
                for _ in 0..count {
                    self.loop_iterations += 1;
                    if self.loop_iterations > self.limits.max_loop_iterations {
                        return Err(runtime(
                            RuntimeErrorKind::LoopLimit,
                            span,
                            self.instructions,
                            "loop iteration limit exhausted",
                        ));
                    }
                    self.run_program(body)?;
                }
            }
            Stmt::Order { name, args, .. } => self.emit_order(name, args, span)?,
            Stmt::Event { value, .. } => {
                if self.events.len() >= self.limits.max_output_events {
                    return Err(runtime(
                        RuntimeErrorKind::OutputLimit,
                        span,
                        self.instructions,
                        "script event output limit exhausted",
                    ));
                }
                let name = self.eval(value)?.text(span)?;
                if name.is_empty() || name.len() > 128 {
                    return Err(runtime(
                        RuntimeErrorKind::InvalidOrder,
                        span,
                        self.instructions,
                        "event name must contain 1..128 characters",
                    ));
                }
                self.events.push(ScriptEvent {
                    side: self.input.side,
                    name,
                });
            }
            Stmt::Victory { value, .. } => self.victory = Some(self.eval(value)?.bool(span)?),
        }
        Ok(())
    }
    fn emit_order(
        &mut self,
        name: &str,
        args: &[Expr],
        span: SourceSpan,
    ) -> Result<(), ScriptRuntimeError> {
        if self.input.side == Side::Neutral {
            return Err(runtime(
                RuntimeErrorKind::InvalidOrder,
                span,
                self.instructions,
                "neutral scripts cannot issue commands",
            ));
        }
        if self.commands.len() >= self.limits.max_output_commands {
            return Err(runtime(
                RuntimeErrorKind::OutputLimit,
                span,
                self.instructions,
                "script command output limit exhausted",
            ));
        }
        let values = args
            .iter()
            .map(|e| self.eval(e))
            .collect::<Result<Vec<_>, _>>()?;
        let unit_value = |v: &Value| -> Result<UnitId, ScriptRuntimeError> {
            let n = v.number(span)?;
            if n.fract() != 0.0 || n < 0.0 || n > u32::MAX as f64 {
                return Err(runtime(
                    RuntimeErrorKind::InvalidOrder,
                    span,
                    self.instructions,
                    "unit id must be a non-negative integer",
                ));
            }
            Ok(UnitId(n as u32))
        };
        let order = match name {
            "stop" if values.len() == 1 => Order::Stop {
                units: vec![unit_value(&values[0])?],
            },
            "move" if values.len() == 3 => {
                let u = unit_value(&values[0])?;
                Order::Move {
                    units: vec![u],
                    gesture: self.commands.len() as u64 + 1,
                    goal: [values[1].number(span)?, values[2].number(span)?],
                    route: RoutePolicy::Shortest,
                    direction: MoveDirection::Forward,
                    facing: None,
                }
            }
            "engagement" if values.len() == 2 => {
                let u = unit_value(&values[0])?;
                let policy = match &values[1] {
                    Value::Text(s) if s == "fire_at_will" => Engagement::FireAtWill,
                    Value::Text(s) if s == "return_fire_only" => Engagement::ReturnFireOnly,
                    _ => {
                        return Err(runtime(
                            RuntimeErrorKind::InvalidOrder,
                            span,
                            self.instructions,
                            "engagement policy must be fire_at_will or return_fire_only",
                        ))
                    }
                };
                Order::SetEngagement {
                    units: vec![u],
                    policy,
                }
            }
            "attack" if values.len() == 2 => {
                let unit = unit_value(&values[0])?;
                let target = unit_value(&values[1])?;
                let observed = ObservedTargetId(target.0);
                if self
                    .input
                    .observation
                    .identified
                    .iter()
                    .all(|enemy| enemy.id != observed)
                {
                    return Err(runtime(
                        RuntimeErrorKind::InvalidOrder,
                        span,
                        self.instructions,
                        "attack target is outside the side's observation",
                    ));
                }
                Order::Attack {
                    units: vec![unit],
                    target: TargetRef::Identified { id: observed },
                }
            }
            _ => {
                return Err(runtime(
                    RuntimeErrorKind::InvalidOrder,
                    span,
                    self.instructions,
                    "unknown order or wrong argument count",
                ))
            }
        };
        self.commands.push(ScriptCommand {
            side: self.input.side,
            slot: self.input.slot.clone(),
            order,
            queued: false,
        });
        Ok(())
    }
    fn eval(&mut self, expr: &Expr) -> Result<Value, ScriptRuntimeError> {
        let span = expr_span(expr);
        self.step(span)?;
        match expr {
            Expr::Literal(v, _) => Ok(v.clone()),
            Expr::Variable(name, _) => self
                .variables
                .iter()
                .rev()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
                .ok_or_else(|| {
                    runtime(
                        RuntimeErrorKind::TypeError,
                        span,
                        self.instructions,
                        "unknown variable",
                    )
                }),
            Expr::Call { name, args, .. } => self.call(name, args, span),
            Expr::Unary { op, value, .. } => match op {
                UnaryOp::Neg => Ok(Value::Number(-self.eval(value)?.number(span)?)),
                UnaryOp::Not => Ok(Value::Bool(!self.eval(value)?.bool(span)?)),
            },
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.eval(left)?;
                if matches!(op, BinaryOp::And) && !l.bool(span)? {
                    return Ok(Value::Bool(false));
                }
                if matches!(op, BinaryOp::Or) && l.bool(span)? {
                    return Ok(Value::Bool(true));
                }
                let r = self.eval(right)?;
                match op {
                    BinaryOp::Add => Ok(Value::Number(l.number(span)? + r.number(span)?)),
                    BinaryOp::Sub => Ok(Value::Number(l.number(span)? - r.number(span)?)),
                    BinaryOp::Mul => Ok(Value::Number(l.number(span)? * r.number(span)?)),
                    BinaryOp::Div => {
                        let d = r.number(span)?;
                        if d == 0.0 {
                            return Err(runtime(
                                RuntimeErrorKind::TypeError,
                                span,
                                self.instructions,
                                "division by zero",
                            ));
                        }
                        Ok(Value::Number(l.number(span)? / d))
                    }
                    BinaryOp::Eq => Ok(Value::Bool(l == r)),
                    BinaryOp::Ne => Ok(Value::Bool(l != r)),
                    BinaryOp::Lt => Ok(Value::Bool(l.number(span)? < r.number(span)?)),
                    BinaryOp::Le => Ok(Value::Bool(l.number(span)? <= r.number(span)?)),
                    BinaryOp::Gt => Ok(Value::Bool(l.number(span)? > r.number(span)?)),
                    BinaryOp::Ge => Ok(Value::Bool(l.number(span)? >= r.number(span)?)),
                    BinaryOp::And => Ok(Value::Bool(l.bool(span)? && r.bool(span)?)),
                    BinaryOp::Or => Ok(Value::Bool(l.bool(span)? || r.bool(span)?)),
                }
            }
        }
    }
    fn call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: SourceSpan,
    ) -> Result<Value, ScriptRuntimeError> {
        let values = args
            .iter()
            .map(|e| self.eval(e))
            .collect::<Result<Vec<_>, _>>()?;
        match (name, values.as_slice()) {
            ("tick", []) => Ok(Value::Number(self.input.tick as f64)),
            ("own_count", []) => Ok(Value::Number(self.input.observation.own.len() as f64)),
            ("enemy_count", []) => {
                Ok(Value::Number(self.input.observation.identified.len() as f64))
            }
            ("contact_count", []) => {
                Ok(Value::Number(self.input.observation.contacts.len() as f64))
            }
            ("audible_count", []) => Ok(Value::Number(self.input.observation.audible.len() as f64)),
            ("own_id", [Value::Number(index)]) => {
                let i = checked_index(
                    *index,
                    self.input.observation.own.len(),
                    span,
                    self.instructions,
                )?;
                Ok(Value::Number(self.input.observation.own[i].id.0 as f64))
            }
            ("own_x", [Value::Number(index)]) => {
                let i = checked_index(
                    *index,
                    self.input.observation.own.len(),
                    span,
                    self.instructions,
                )?;
                Ok(Value::Number(self.input.observation.own[i].position[0]))
            }
            ("own_y", [Value::Number(index)]) => {
                let i = checked_index(
                    *index,
                    self.input.observation.own.len(),
                    span,
                    self.instructions,
                )?;
                Ok(Value::Number(self.input.observation.own[i].position[1]))
            }
            ("enemy_id", [Value::Number(index)]) => {
                let i = checked_index(
                    *index,
                    self.input.observation.identified.len(),
                    span,
                    self.instructions,
                )?;
                Ok(Value::Number(
                    self.input.observation.identified[i].id.0 as f64,
                ))
            }
            ("enemy_x", [Value::Number(index)]) => {
                let i = checked_index(
                    *index,
                    self.input.observation.identified.len(),
                    span,
                    self.instructions,
                )?;
                Ok(Value::Number(
                    self.input.observation.identified[i].position[0],
                ))
            }
            ("enemy_y", [Value::Number(index)]) => {
                let i = checked_index(
                    *index,
                    self.input.observation.identified.len(),
                    span,
                    self.instructions,
                )?;
                Ok(Value::Number(
                    self.input.observation.identified[i].position[1],
                ))
            }
            ("side", []) => Ok(Value::Text(
                match self.input.side {
                    Side::Blue => "blue",
                    Side::Red => "red",
                    Side::Neutral => "neutral",
                }
                .into(),
            )),
            ("random", [Value::Number(max)]) => {
                if !max.is_finite() || *max <= 0.0 || max.fract() != 0.0 || *max > u32::MAX as f64 {
                    return Err(runtime(
                        RuntimeErrorKind::TypeError,
                        span,
                        self.instructions,
                        "random bound must be a positive integer",
                    ));
                }
                let draw = mix(self.input.rng_seed.wrapping_add(self.rng_counter));
                self.rng_counter = self.rng_counter.wrapping_add(1);
                Ok(Value::Number((draw % *max as u64) as f64))
            }
            _ => Err(runtime(
                RuntimeErrorKind::TypeError,
                span,
                self.instructions,
                "host function is unavailable or has wrong argument count",
            )),
        }
    }
}

fn checked_index(
    value: f64,
    len: usize,
    span: SourceSpan,
    instructions: u32,
) -> Result<usize, ScriptRuntimeError> {
    if !value.is_finite() || value.fract() != 0.0 || value < 0.0 || value >= len as f64 {
        return Err(runtime(
            RuntimeErrorKind::InvalidOrder,
            span,
            instructions,
            "observation index is outside the visible list",
        ));
    }
    Ok(value as usize)
}

fn runtime(
    kind: RuntimeErrorKind,
    span: SourceSpan,
    instructions: u32,
    message: &str,
) -> ScriptRuntimeError {
    ScriptRuntimeError {
        kind,
        span: Some(span),
        instructions,
        message: message.into(),
    }
}
fn stmt_span(stmt: &Stmt) -> SourceSpan {
    match stmt {
        Stmt::Let { span, .. }
        | Stmt::If { span, .. }
        | Stmt::Repeat { span, .. }
        | Stmt::Order { span, .. }
        | Stmt::Event { span, .. }
        | Stmt::Victory { span, .. } => *span,
    }
}
fn expr_span(expr: &Expr) -> SourceSpan {
    match expr {
        Expr::Literal(_, s)
        | Expr::Variable(_, s)
        | Expr::Call { span: s, .. }
        | Expr::Unary { span: s, .. }
        | Expr::Binary { span: s, .. } => *s,
    }
}

#[derive(Clone, Debug)]
enum TokenKind {
    Ident(String),
    Number(f64),
    String(String),
    Symbol(char),
    Op(&'static str),
    Eof,
}
#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    span: SourceSpan,
}

struct Lexer<'a> {
    source: &'a str,
    offset: usize,
    line: usize,
    column: usize,
}
impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
            line: 1,
            column: 1,
        }
    }
    fn lex(mut self) -> Result<Vec<Token>, ScriptDiagnostic> {
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
                continue;
            }
            if c == '/' && self.peek_n(1) == Some('/') {
                while let Some(c) = self.bump() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
            let start = self.mark();
            if c.is_ascii_alphabetic() || c == '_' {
                let mut s = String::new();
                while self
                    .peek()
                    .is_some_and(|x| x.is_ascii_alphanumeric() || x == '_')
                {
                    s.push(self.bump().unwrap());
                }
                out.push(Token {
                    kind: TokenKind::Ident(s),
                    span: self.span(start),
                });
                continue;
            }
            if c.is_ascii_digit()
                || (c == '.' && self.peek_n(1).is_some_and(|x| x.is_ascii_digit()))
            {
                let mut s = String::new();
                while self.peek().is_some_and(|x| x.is_ascii_digit()) {
                    s.push(self.bump().unwrap());
                }
                if self.peek() == Some('.') {
                    s.push(self.bump().unwrap());
                    while self.peek().is_some_and(|x| x.is_ascii_digit()) {
                        s.push(self.bump().unwrap());
                    }
                }
                if self.peek().is_some_and(|x| x == 'e' || x == 'E') {
                    s.push(self.bump().unwrap());
                    if self.peek().is_some_and(|x| x == '+' || x == '-') {
                        s.push(self.bump().unwrap());
                    }
                    let exponent_start = s.len();
                    while self.peek().is_some_and(|x| x.is_ascii_digit()) {
                        s.push(self.bump().unwrap());
                    }
                    if s.len() == exponent_start {
                        return Err(diag("number", self.span(start), "invalid number exponent"));
                    }
                }
                let n = s
                    .parse::<f64>()
                    .map_err(|_| diag("number", self.span(start), "invalid number"))?;
                if !n.is_finite() {
                    return Err(diag("number", self.span(start), "number is not finite"));
                }
                out.push(Token {
                    kind: TokenKind::Number(n),
                    span: self.span(start),
                });
                continue;
            }
            if c == '"' {
                self.bump();
                let mut s = String::new();
                loop {
                    match self.bump() {
                        Some('"') => break,
                        Some('\\') => match self.bump() {
                            Some('"') => s.push('"'),
                            Some('\\') => s.push('\\'),
                            Some('n') => s.push('\n'),
                            Some('r') => s.push('\r'),
                            Some('t') => s.push('\t'),
                            _ => {
                                return Err(diag(
                                    "string",
                                    self.span(start),
                                    "unsupported string escape",
                                ))
                            }
                        },
                        Some(ch) if ch != '\n' => s.push(ch),
                        _ => return Err(diag("string", self.span(start), "unterminated string")),
                    }
                }
                out.push(Token {
                    kind: TokenKind::String(s),
                    span: self.span(start),
                });
                continue;
            }
            let op = match (c, self.peek_n(1)) {
                ('=', Some('=')) => Some(("==", 2)),
                ('!', Some('=')) => Some(("!=", 2)),
                ('<', Some('=')) => Some(("<=", 2)),
                ('>', Some('=')) => Some((">=", 2)),
                ('&', Some('&')) => Some(("&&", 2)),
                ('|', Some('|')) => Some(("||", 2)),
                _ => None,
            };
            if let Some((name, n)) = op {
                for _ in 0..n {
                    self.bump();
                }
                out.push(Token {
                    kind: TokenKind::Op(name),
                    span: self.span(start),
                });
                continue;
            }
            if "{}();,=+-*/!<>".contains(c) {
                self.bump();
                out.push(Token {
                    kind: if c == '=' {
                        TokenKind::Op("=")
                    } else {
                        TokenKind::Symbol(c)
                    },
                    span: self.span(start),
                });
                continue;
            }
            return Err(diag(
                "character",
                self.span(start),
                "character is not allowed in scripts",
            ));
        }
        out.push(Token {
            kind: TokenKind::Eof,
            span: SourceSpan {
                start: self.offset,
                end: self.offset,
                line: self.line,
                column: self.column,
            },
        });
        Ok(out)
    }
    fn mark(&self) -> (usize, usize, usize) {
        (self.offset, self.line, self.column)
    }
    fn span(&self, start: (usize, usize, usize)) -> SourceSpan {
        SourceSpan {
            start: start.0,
            end: self.offset,
            line: start.1,
            column: start.2,
        }
    }
    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }
    fn peek_n(&self, n: usize) -> Option<char> {
        self.source[self.offset..].chars().nth(n)
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.offset += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1
        } else {
            self.column += 1
        };
        Some(c)
    }
}
fn diag(code: &'static str, span: SourceSpan, message: impl Into<String>) -> ScriptDiagnostic {
    ScriptDiagnostic {
        code,
        span,
        message: message.into(),
    }
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    nesting: usize,
}
impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            at: 0,
            nesting: 0,
        }
    }
    fn program(&mut self) -> Result<Vec<Stmt>, Vec<ScriptDiagnostic>> {
        let mut out = Vec::new();
        let mut e = Vec::new();
        while !self.is_eof() {
            match self.stmt() {
                Ok(s) => out.push(s),
                Err(d) => {
                    e.push(d);
                    self.recover()
                }
            }
        }
        if e.is_empty() {
            Ok(out)
        } else {
            Err(e)
        }
    }
    fn stmt(&mut self) -> Result<Stmt, ScriptDiagnostic> {
        let t = self.next();
        let (kw, span) = match &t.kind {
            TokenKind::Ident(s) => (s.clone(), t.span),
            _ => return Err(diag("statement", t.span, "expected a statement keyword")),
        };
        match kw.as_str() {
            "let" => {
                let name = self.ident()?;
                self.expect_op("=")?;
                let value = self.expr()?;
                self.expect_symbol(';')?;
                Ok(Stmt::Let { name, value, span })
            }
            "if" => {
                let condition = self.expr()?;
                let then_body = self.block()?;
                let else_body = if self.peek_ident("else") {
                    self.next();
                    self.block()?
                } else {
                    Vec::new()
                };
                Ok(Stmt::If {
                    condition,
                    then_body,
                    else_body,
                    span,
                })
            }
            "repeat" => {
                let count = self.expr()?;
                let body = self.block()?;
                Ok(Stmt::Repeat { count, body, span })
            }
            "order" => {
                let name = self.ident()?;
                let args = self.call_args()?;
                self.expect_symbol(';')?;
                Ok(Stmt::Order { name, args, span })
            }
            "event" => {
                let value = self.expr()?;
                self.expect_symbol(';')?;
                Ok(Stmt::Event { value, span })
            }
            "victory" | "return" => {
                let value = self.expr()?;
                // `return condition` is accepted as a concise victory script;
                // the canonical authored form uses `victory(condition);`.
                if self.peek_symbol(';') {
                    self.next();
                } else if kw == "victory" {
                    return Err(diag("syntax", span, "expected ';'"));
                }
                Ok(Stmt::Victory { value, span })
            }
            _ => Err(diag("statement", span, "unknown statement")),
        }
    }
    fn block(&mut self) -> Result<Vec<Stmt>, ScriptDiagnostic> {
        self.nesting += 1;
        if self.nesting > MAX_PARSE_NESTING {
            self.nesting -= 1;
            return Err(diag(
                "nesting_limit",
                self.tokens
                    .get(self.at)
                    .map(|token| token.span)
                    .unwrap_or(SourceSpan {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    }),
                "script nesting exceeds the compile limit",
            ));
        }
        self.expect_symbol('{')?;
        let mut out = Vec::new();
        while !self.peek_symbol('}') && !self.is_eof() {
            out.push(self.stmt()?);
        }
        self.expect_symbol('}')?;
        self.nesting -= 1;
        Ok(out)
    }
    fn expr(&mut self) -> Result<Expr, ScriptDiagnostic> {
        self.or()
    }
    fn or(&mut self) -> Result<Expr, ScriptDiagnostic> {
        self.binary(Self::and, &[TokenKind::Op("||")], BinaryOp::Or)
    }
    fn and(&mut self) -> Result<Expr, ScriptDiagnostic> {
        self.binary(Self::equality, &[TokenKind::Op("&&")], BinaryOp::And)
    }
    fn equality(&mut self) -> Result<Expr, ScriptDiagnostic> {
        self.binary(
            Self::comparison,
            &[TokenKind::Op("=="), TokenKind::Op("!=")],
            BinaryOp::Eq,
        )
    }
    fn comparison(&mut self) -> Result<Expr, ScriptDiagnostic> {
        let mut left = self.term()?;
        loop {
            let op = match self.peek_op() {
                Some("<") => Some(BinaryOp::Lt),
                Some("<=") => Some(BinaryOp::Le),
                Some(">") => Some(BinaryOp::Gt),
                Some(">=") => Some(BinaryOp::Ge),
                _ => None,
            };
            if let Some(op) = op {
                let span = self.next().span;
                let right = self.term()?;
                left = Expr::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(left)
    }
    fn term(&mut self) -> Result<Expr, ScriptDiagnostic> {
        let mut left = self.factor()?;
        loop {
            let op = match self.peek_symbol_any() {
                Some('+') => Some(BinaryOp::Add),
                Some('-') => Some(BinaryOp::Sub),
                _ => None,
            };
            if let Some(op) = op {
                let span = self.next().span;
                let right = self.factor()?;
                left = Expr::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(left)
    }
    fn factor(&mut self) -> Result<Expr, ScriptDiagnostic> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek_symbol_any() {
                Some('*') => Some(BinaryOp::Mul),
                Some('/') => Some(BinaryOp::Div),
                _ => None,
            };
            if let Some(op) = op {
                let span = self.next().span;
                let right = self.unary()?;
                left = Expr::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(left)
    }
    fn unary(&mut self) -> Result<Expr, ScriptDiagnostic> {
        if let Some(c) = self.peek_symbol_any() {
            if c == '-' || c == '!' {
                let span = self.next().span;
                return Ok(Expr::Unary {
                    op: if c == '-' { UnaryOp::Neg } else { UnaryOp::Not },
                    value: Box::new(self.unary()?),
                    span,
                });
            }
        }
        self.primary()
    }
    fn primary(&mut self) -> Result<Expr, ScriptDiagnostic> {
        let t = self.next();
        match t.kind {
            TokenKind::Number(n) => Ok(Expr::Literal(Value::Number(n), t.span)),
            TokenKind::String(s) => Ok(Expr::Literal(Value::Text(s), t.span)),
            TokenKind::Ident(name) => {
                if name == "true" || name == "false" {
                    return Ok(Expr::Literal(Value::Bool(name == "true"), t.span));
                }
                if self.peek_symbol('(') {
                    let args = self.call_args()?;
                    Ok(Expr::Call {
                        name,
                        args,
                        span: t.span,
                    })
                } else {
                    Ok(Expr::Variable(name, t.span))
                }
            }
            TokenKind::Symbol('(') => {
                self.nesting += 1;
                if self.nesting > MAX_PARSE_NESTING {
                    self.nesting -= 1;
                    return Err(diag(
                        "nesting_limit",
                        t.span,
                        "script nesting exceeds the compile limit",
                    ));
                }
                let e = self.expr()?;
                self.expect_symbol(')')?;
                self.nesting -= 1;
                Ok(e)
            }
            _ => Err(diag("expression", t.span, "expected a value")),
        }
    }
    fn binary<F>(
        &mut self,
        next: F,
        ops: &[TokenKind],
        default: BinaryOp,
    ) -> Result<Expr, ScriptDiagnostic>
    where
        F: Fn(&mut Self) -> Result<Expr, ScriptDiagnostic>,
    {
        let mut left = next(self)?;
        loop {
            let p = self.peek_op();
            let found = ops
                .iter()
                .any(|o| matches!((o,p),(TokenKind::Op(a),Some(b)) if *a==b));
            if !found {
                break;
            }
            let tok = self.next();
            let op = match tok.kind {
                TokenKind::Op("&&") => BinaryOp::And,
                TokenKind::Op("||") => BinaryOp::Or,
                TokenKind::Op("!=") => BinaryOp::Ne,
                _ => default,
            };
            let right = next(self)?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span: tok.span,
            };
        }
        Ok(left)
    }
    fn call_args(&mut self) -> Result<Vec<Expr>, ScriptDiagnostic> {
        self.expect_symbol('(')?;
        let mut out = Vec::new();
        if !self.peek_symbol(')') {
            loop {
                out.push(self.expr()?);
                if self.peek_symbol(')') {
                    break;
                }
                self.expect_symbol(',')?;
            }
        }
        self.expect_symbol(')')?;
        Ok(out)
    }
    fn ident(&mut self) -> Result<String, ScriptDiagnostic> {
        let t = self.next();
        match t.kind {
            TokenKind::Ident(s) => Ok(s),
            _ => Err(diag("identifier", t.span, "expected identifier")),
        }
    }
    fn expect_symbol(&mut self, c: char) -> Result<(), ScriptDiagnostic> {
        let t = self.next();
        if matches!(t.kind,TokenKind::Symbol(x) if x==c) {
            Ok(())
        } else {
            Err(diag("syntax", t.span, format!("expected '{c}'")))
        }
    }
    fn expect_op(&mut self, s: &'static str) -> Result<(), ScriptDiagnostic> {
        let t = self.next();
        if matches!(t.kind,TokenKind::Op(x) if x==s) {
            Ok(())
        } else {
            Err(diag("syntax", t.span, format!("expected '{s}'")))
        }
    }
    fn peek_symbol(&self, c: char) -> bool {
        matches!(self.tokens.get(self.at).map(|t|&t.kind),Some(TokenKind::Symbol(x)) if *x==c)
    }
    fn peek_symbol_any(&self) -> Option<char> {
        match self.tokens.get(self.at).map(|t| &t.kind) {
            Some(TokenKind::Symbol(x)) => Some(*x),
            _ => None,
        }
    }
    fn peek_ident(&self, s: &str) -> bool {
        matches!(self.tokens.get(self.at).map(|t|&t.kind),Some(TokenKind::Ident(x)) if x==s)
    }
    fn peek_op(&self) -> Option<&'static str> {
        match self.tokens.get(self.at).map(|t| &t.kind) {
            Some(TokenKind::Op(s)) => Some(*s),
            _ => None,
        }
    }
    fn next(&mut self) -> Token {
        let t = self.tokens.get(self.at).cloned().unwrap_or(Token {
            kind: TokenKind::Eof,
            span: SourceSpan {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });
        self.at += 1;
        t
    }
    fn is_eof(&self) -> bool {
        matches!(
            self.tokens.get(self.at).map(|t| &t.kind),
            Some(TokenKind::Eof) | None
        )
    }
    fn recover(&mut self) {
        while !self.is_eof() {
            if self.peek_symbol(';') {
                self.next();
                break;
            }
            if self.peek_symbol('}') {
                break;
            }
            self.next();
        }
    }
}
