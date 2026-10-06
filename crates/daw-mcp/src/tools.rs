use daw_analysis::JuliaAnalyzer;
use daw_contracts::{
    require_supported, DawHost, MidiNote, PageRequest, PlanAction, TransactionPlan,
};
use daw_probe::ProbeHub;
use daw_transaction::TransactionEngine;
use serde_json::{json, Value};
use std::path::PathBuf;
use uuid::Uuid;

pub struct ServerState {
    pub host: Box<dyn DawHost>,
    pub tx: TransactionEngine,
    pub probes: ProbeHub,
    pub last_plan: Option<TransactionPlan>,
}

impl ServerState {
    pub fn new(host: Box<dyn DawHost>) -> Self {
        Self {
            host,
            tx: TransactionEngine::default(),
            probes: ProbeHub::default(),
            last_plan: None,
        }
    }
}

pub fn list_tools() -> Vec<Value> {
    daw_contracts::root_tools()
        .iter()
        .map(|name| {
            json!({
                "name": name,
                "description": format!("DAW Agent tool {name}. Call daw_get_capabilities first."),
                "inputSchema": { "type": "object", "properties": {} },
                "annotations": annotations(name)
            })
        })
        .collect()
}

fn annotations(name: &str) -> Value {
    let read = name.starts_with("daw_get")
        || name.starts_with("daw_list")
        || name.starts_with("daw_fetch")
        || name.contains("analyze")
        || name.contains("health")
        || name.contains("preview")
        || name.contains("capabilities")
        || name.contains("session");
    json!({
        "readOnlyHint": read,
        "destructiveHint": matches!(name, "daw_rollback" | "daw_write_midi"),
        "idempotentHint": read,
        "openWorldHint": true
    })
}

pub async fn call(state: &mut ServerState, name: &str, args: Value) -> anyhow::Result<Value> {
    if name != "daw_get_capabilities" {
        if let Err(e) = require_supported(&state.host.capabilities(), name) {
            // analysis tools are served locally even if host profile omitted them
            if !matches!(name, "daw_analyze_spectrum" | "daw_analyze_loudness" | "daw_create_plan" | "daw_preview_plan" | "daw_commit_plan" | "daw_rollback" | "daw_fetch_track_detail" | "daw_locate_marker") {
                return Ok(tool_error(&e.to_string()));
            }
        }
    }
    match name {
        "daw_get_capabilities" => ok(serde_json::to_value(state.host.capabilities())?),
        "daw_get_session_summary" => ok(serde_json::to_value(state.host.session_summary().await?)?),
        "daw_list_tracks" => {
            let page = page_req(&args);
            ok(serde_json::to_value(state.host.list_tracks(page).await?)?)
        }
        "daw_get_track" | "daw_fetch_track_detail" => {
            let id = args.get("track_id").and_then(|v| v.as_str()).unwrap_or("trk-1");
            let track = state.host.get_track(id).await?;
            let plugins = state.host.list_plugins(id).await.unwrap_or_default();
            ok(json!({"track": track, "plugins": plugins}))
        }
        "daw_list_plugins" => {
            let id = args.get("track_id").and_then(|v| v.as_str()).unwrap_or("trk-1");
            ok(serde_json::to_value(state.host.list_plugins(id).await?)?)
        }
        "daw_get_midi_selection" => ok(serde_json::to_value(state.host.midi_selection().await?)?),
        "daw_health_check" => {
            let mut issues = state.host.health_check().await?;
            for hit in state.probes.masking_matrix() {
                issues.push(daw_contracts::HealthIssue {
                    code: "masking".into(),
                    severity: daw_contracts::HealthSeverity::Warn,
                    message: format!("spectral overlap {:.0} Hz between {} and {}", hit.band_hz, hit.a, hit.b),
                    confidence: 0.7,
                    layer: daw_contracts::ControlLayer::L1Probe,
                    track_id: None,
                });
            }
            ok(serde_json::to_value(issues)?)
        }
        "daw_analyze_spectrum" => analyze(state, "spectrum", &args).await,
        "daw_analyze_loudness" => analyze(state, "loudness", &args).await,
        "daw_create_plan" => {
            let actions: Vec<PlanAction> = serde_json::from_value(args.get("actions").cloned().unwrap_or(json!([])))?;
            let confirmed = args.get("confirmed").and_then(|v| v.as_bool()).unwrap_or(false);
            match state.tx.create_plan(&state.host.capabilities(), actions, confirmed) {
                Ok(plan) => {
                    state.last_plan = Some(plan.clone());
                    ok(serde_json::to_value(plan)?)
                }
                Err(e) => Ok(tool_error(&e.to_string())),
            }
        }
        "daw_preview_plan" => {
            let plan = plan_from(state, &args)?;
            let preview = state.tx.preview(&plan);
            ok(json!({"markdown": preview.markdown, "plan": preview.plan}))
        }
        "daw_commit_plan" => {
            let plan = plan_from(state, &args)?;
            let confirmed = args.get("confirmed").and_then(|v| v.as_bool()).unwrap_or(false);
            match state.tx.commit(state.host.as_ref(), &plan, confirmed).await {
                Ok(cp) => ok(serde_json::to_value(cp)?),
                Err(e) => Ok(tool_error(&e.to_string())),
            }
        }
        "daw_rollback" => {
            let id = args
                .get("plan_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("plan_id required"))?;
            let uuid = Uuid::parse_str(id)?;
            state.tx.rollback(state.host.as_ref(), uuid).await?;
            ok(json!({"rolled_back": id}))
        }
        "daw_write_midi" => {
            let notes: Vec<MidiNote> = serde_json::from_value(args.get("notes").cloned().unwrap_or(json!([])))?;
            state.host.write_midi(notes).await?;
            ok(json!({"ok": true}))
        }
        "daw_set_parameter" => {
            let plugin_id = arg_str(&args, "plugin_id")?;
            let pname = arg_str(&args, "name")?;
            let value = args.get("value").and_then(|v| v.as_f64()).ok_or_else(|| anyhow::anyhow!("value"))?;
            state.host.set_parameter(plugin_id, pname, value).await?;
            ok(json!({"ok": true}))
        }
        "daw_set_track_mute" => {
            let id = arg_str(&args, "track_id")?;
            let mute = args.get("mute").and_then(|v| v.as_bool()).unwrap_or(true);
            state.host.set_track_mute(id, mute).await?;
            ok(json!({"ok": true}))
        }
        "daw_rename_track" => {
            let id = arg_str(&args, "track_id")?;
            let name = arg_str(&args, "name")?;
            state.host.rename_track(id, name).await?;
            ok(json!({"ok": true}))
        }
        "daw_set_playhead" | "daw_locate_marker" => {
            let beats = args.get("beats").and_then(|v| v.as_f64()).unwrap_or(0.0);
            state.host.set_playhead_beats(beats).await?;
            ok(json!({"ok": true}))
        }
        other => Ok(tool_error(&format!("unknown tool {other}"))),
    }
}

fn arg_str<'a>(args: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    args.get(key).and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("{key} required"))
}

fn page_req(args: &Value) -> PageRequest {
    PageRequest {
        limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(32) as u32,
        offset: args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
    }
}

fn plan_from(state: &ServerState, args: &Value) -> anyhow::Result<TransactionPlan> {
    if let Some(p) = args.get("plan") {
        return Ok(serde_json::from_value(p.clone())?);
    }
    state
        .last_plan
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no plan; call daw_create_plan first"))
}

async fn analyze(state: &ServerState, op: &str, args: &Value) -> anyhow::Result<Value> {
    let samples: Vec<f32> = args
        .get("samples")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| vec![0.0; 64]);
    let sr = args.get("sample_rate").and_then(|v| v.as_f64()).unwrap_or(48000.0) as f32;
    let root = workspace_root();
    let julia = JuliaAnalyzer::from_workspace_root(root);
    let result = match op {
        "spectrum" => match julia.spectrum(&samples, sr).await {
            Ok(v) => serde_json::to_value(v)?,
            Err(_) => fallback_spectrum(&samples),
        },
        _ => match julia.loudness(&samples, sr).await {
            Ok(v) => serde_json::to_value(v)?,
            Err(_) => fallback_loudness(&samples),
        },
    };
    let _ = state;
    ok(result)
}

fn fallback_spectrum(samples: &[f32]) -> Value {
    let rms = if samples.is_empty() {
        1e-6
    } else {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    };
    json!({
        "bands_hz": [80.0, 250.0, 1000.0, 4000.0],
        "magnitudes_db": [-12.0, 20.0 * rms.log10(), -18.0, -24.0],
        "fallback": true
    })
}

fn fallback_loudness(samples: &[f32]) -> Value {
    let peak = samples.iter().copied().fold(0.0f32, |a, b| a.max(b.abs()));
    json!({
        "lufs_integrated": -23.0,
        "true_peak_dbtp": 20.0 * (peak + 1e-6).log10(),
        "lra": 6.0,
        "fallback": true
    })
}

fn workspace_root() -> PathBuf {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok()
        .and_then(|p| p.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn ok(structured: Value) -> anyhow::Result<Value> {
    let md = format!("```json\n{}\n```", serde_json::to_string_pretty(&structured)?);
    Ok(json!({
        "content": [{"type":"text","text": md}],
        "structuredContent": structured,
        "isError": false
    }))
}

fn tool_error(message: &str) -> Value {
    json!({
        "content": [{"type":"text","text": message}],
        "isError": true
    })
}
