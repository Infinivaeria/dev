use crate::voice::{AvatarState, VoiceBand};
use rhai::{AST, Dynamic, Engine, Map, Scope};
use std::fmt;
use std::path::{Path, PathBuf};

const ALLOWED_OUTPUTS: [&str; 9] = [
    "pose",
    "band",
    "bounce",
    "scale",
    "mirror",
    "blink",
    "gate_db",
    "loud_delta_db",
    "gain_db",
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScriptInput {
    pub frame: u64,
    pub time: f32,
    pub dt: f32,
    pub db: f32,
    pub adjusted_db: f32,
    pub pitch_hz: Option<f32>,
    pub band: VoiceBand,
    pub pose: AvatarState,
    pub muted: bool,
    pub gate_db: f32,
    pub loud_delta_db: f32,
}

impl Default for AvatarState {
    fn default() -> Self {
        Self::Idle
    }
}

impl Default for VoiceBand {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptControls {
    pub pose: Option<AvatarState>,
    pub band: Option<VoiceBand>,
    pub bounce: Option<f32>,
    pub scale: Option<f32>,
    pub mirror: Option<bool>,
    pub blink: Option<bool>,
    pub gate_db: Option<f32>,
    pub loud_delta_db: Option<f32>,
    pub gain_db: Option<f32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptError {
    Read(PathBuf, String),
    Compile(String),
    MissingUpdate,
    Runtime(String),
    InvalidOutput(String),
}

impl fmt::Display for ScriptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(path, reason) => {
                write!(formatter, "could not read {}: {reason}", path.display())
            }
            Self::Compile(reason) => write!(formatter, "script compile failed: {reason}"),
            Self::MissingUpdate => {
                formatter.write_str("script must define update(input) returning an override map")
            }
            Self::Runtime(reason) => write!(formatter, "script runtime failed: {reason}"),
            Self::InvalidOutput(reason) => {
                write!(formatter, "script returned invalid controls: {reason}")
            }
        }
    }
}

impl std::error::Error for ScriptError {}

pub struct ScriptRuntime {
    engine: Engine,
    ast: AST,
    scope: Scope<'static>,
}

impl ScriptRuntime {
    pub fn load(path: &Path) -> Result<Self, ScriptError> {
        let source = std::fs::read_to_string(path)
            .map_err(|error| ScriptError::Read(path.to_path_buf(), error.to_string()))?;
        Self::from_source(&source)
    }

    pub fn from_source(source: &str) -> Result<Self, ScriptError> {
        let mut engine = Engine::new();
        engine.set_max_expr_depths(32, 32);
        engine.set_max_call_levels(24);
        engine.set_max_operations(20_000);
        engine.set_max_variables(256);
        engine.set_max_string_size(65_536);
        engine.set_max_array_size(1_024);
        engine.set_max_map_size(128);
        engine.set_strict_variables(true);
        engine.disable_symbol("eval");
        let ast = engine
            .compile(source)
            .map_err(|error| ScriptError::Compile(error.to_string()))?;
        if ast
            .iter_functions()
            .all(|function| function.name != "update")
        {
            return Err(ScriptError::MissingUpdate);
        }
        Ok(Self {
            engine,
            ast,
            scope: Scope::new(),
        })
    }

    pub fn update(&mut self, input: ScriptInput) -> Result<ScriptControls, ScriptError> {
        let mut data = Map::new();
        data.insert("frame".into(), (input.frame as i64).into());
        data.insert("time".into(), (input.time as f64).into());
        data.insert("dt".into(), (input.dt as f64).into());
        data.insert("db".into(), (input.db as f64).into());
        data.insert("adjusted_db".into(), (input.adjusted_db as f64).into());
        data.insert(
            "pitch_hz".into(),
            input
                .pitch_hz
                .map_or(Dynamic::UNIT, |frequency| (frequency as f64).into()),
        );
        data.insert("band".into(), input.band.cli_name().into());
        data.insert("pose".into(), input.pose.file_stem().into());
        data.insert("muted".into(), input.muted.into());
        data.insert("gate_db".into(), (input.gate_db as f64).into());
        data.insert("loud_delta_db".into(), (input.loud_delta_db as f64).into());
        let output: Dynamic = self
            .engine
            .call_fn(&mut self.scope, &self.ast, "update", (data,))
            .map_err(|error| ScriptError::Runtime(error.to_string()))?;
        parse_controls(output)
    }
}

fn parse_controls(value: Dynamic) -> Result<ScriptControls, ScriptError> {
    if value.is_unit() {
        return Ok(ScriptControls::default());
    }
    let map = value.try_cast::<Map>().ok_or_else(|| {
        ScriptError::InvalidOutput("update(input) must return a map or unit".into())
    })?;
    for key in map.keys() {
        if !ALLOWED_OUTPUTS.contains(&key.as_str()) {
            return Err(ScriptError::InvalidOutput(format!("unknown field `{key}`")));
        }
    }
    let mut controls = ScriptControls::default();
    if let Some(value) = map.get("pose") {
        controls.pose = match text(value, "pose")?.as_str() {
            "idle" => Some(AvatarState::Idle),
            "talking" | "talk" => Some(AvatarState::Talking),
            "loud" => Some(AvatarState::Loud),
            "auto" => None,
            other => {
                return Err(ScriptError::InvalidOutput(format!(
                    "pose `{other}` must be auto, idle, talking, or loud"
                )));
            }
        };
    }
    if let Some(value) = map.get("band") {
        controls.band = Some(match text(value, "band")?.as_str() {
            "low" => VoiceBand::Low,
            "mid" => VoiceBand::Mid,
            "high" => VoiceBand::High,
            "global" | "unpitched" | "auto" => VoiceBand::Unknown,
            other => {
                return Err(ScriptError::InvalidOutput(format!(
                    "band `{other}` must be low, mid, high, global, or auto"
                )));
            }
        });
    }
    controls.bounce = optional_float(&map, "bounce", 0.0, 2.0)?;
    controls.scale = optional_float(&map, "scale", 0.2, 1.0)?;
    controls.mirror = optional_bool(&map, "mirror")?;
    controls.blink = optional_bool(&map, "blink")?;
    controls.gate_db = optional_float(&map, "gate_db", -120.0, 0.0)?;
    controls.loud_delta_db = optional_float(&map, "loud_delta_db", 4.0, 120.0)?;
    controls.gain_db = optional_float(&map, "gain_db", -36.0, 36.0)?;
    Ok(controls)
}

fn text(value: &Dynamic, field: &str) -> Result<String, ScriptError> {
    value
        .clone()
        .into_string()
        .map_err(|_| ScriptError::InvalidOutput(format!("field `{field}` must be a string")))
}

fn optional_float(
    map: &Map,
    field: &str,
    minimum: f32,
    maximum: f32,
) -> Result<Option<f32>, ScriptError> {
    let Some(value) = map.get(field) else {
        return Ok(None);
    };
    let number = value
        .as_float()
        .map(|value| value as f32)
        .or_else(|_| value.as_int().map(|value| value as f32))
        .map_err(|_| ScriptError::InvalidOutput(format!("field `{field}` must be numeric")))?;
    if !number.is_finite() || !(minimum..=maximum).contains(&number) {
        return Err(ScriptError::InvalidOutput(format!(
            "field `{field}` must be between {minimum} and {maximum}"
        )));
    }
    Ok(Some(number))
}

fn optional_bool(map: &Map, field: &str) -> Result<Option<bool>, ScriptError> {
    map.get(field)
        .map(|value| {
            value
                .as_bool()
                .map_err(|_| ScriptError::InvalidOutput(format!("field `{field}` must be boolean")))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_are_turing_complete_and_cannot_exceed_the_budget() {
        let mut runtime = ScriptRuntime::from_source(
            r#"
                fn update(input) {
                    let n = 0;
                    let total = 0;
                    while n < 100 {
                        total += n;
                        n += 1;
                    }
                    if total == 4950 && input.band == "mid" {
                        #{ pose: "talking", bounce: 1.5 }
                    } else {
                        #{}
                    }
                }
            "#,
        )
        .unwrap();
        let controls = runtime
            .update(ScriptInput {
                band: VoiceBand::Mid,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(controls.pose, Some(AvatarState::Talking));
        assert_eq!(controls.bounce, Some(1.5));

        let mut infinite = ScriptRuntime::from_source("fn update(input) { loop {} }").unwrap();
        assert!(matches!(
            infinite.update(ScriptInput::default()),
            Err(ScriptError::Runtime(_))
        ));
    }

    #[test]
    fn scripts_validate_types_ranges_and_output_fields() {
        let mut valid = ScriptRuntime::from_source(
            r#"fn update(input) { #{ pose: "loud", band: "global", mirror: true, scale: 0.8 } }"#,
        )
        .unwrap();
        assert_eq!(
            valid.update(ScriptInput::default()).unwrap(),
            ScriptControls {
                pose: Some(AvatarState::Loud),
                band: Some(VoiceBand::Unknown),
                scale: Some(0.8),
                mirror: Some(true),
                ..Default::default()
            }
        );
        for source in [
            "fn update(input) { #{ surprise: 1 } }",
            "fn update(input) { #{ scale: 4 } }",
            "fn update(input) { #{ pose: 4 } }",
            "fn update(input) { 42 }",
        ] {
            assert!(matches!(
                ScriptRuntime::from_source(source)
                    .unwrap()
                    .update(ScriptInput::default()),
                Err(ScriptError::InvalidOutput(_))
            ));
        }
        assert!(matches!(
            ScriptRuntime::from_source("let value = 1;"),
            Err(ScriptError::MissingUpdate)
        ));
    }
}
