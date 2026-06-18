use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Tracer (collector) information from agent's /systeminfo endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tracer {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: i64,
    pub name: String,
    pub host: String,
    pub group_id: Option<i64>,
    pub group_name: Option<String>,
    pub status: String,
    pub tags: Option<String>,
    pub cpu: Option<f64>,
    pub memory: Option<f64>,
    pub rate: Option<f64>,
    pub uptime: Option<i64>,
    pub version: Option<String>,
    pub tracers: Option<Vec<Tracer>>,
    pub token: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgent {
    pub name: String,
    pub host: String,
    pub group_id: Option<i64>,
    pub tags: Option<String>,
    /// Bearer token for agent API authentication
    pub token: Option<String>,
    /// Optional list of sink IDs to deploy to this agent
    pub sink_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgent {
    pub name: Option<String>,
    pub host: Option<String>,
    pub group_id: Option<i64>,
    pub tags: Option<String>,
    pub token: Option<String>,
}

/// Response from agent's /systeminfo endpoint
#[derive(Debug, Deserialize)]
pub struct AgentSystemInfo {
    pub version: String,
    pub build_time: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub collectors: std::collections::HashMap<String, String>,
}

impl AgentSystemInfo {
    /// Convert collectors HashMap to Vec<Tracer>
    pub fn to_tracers(&self) -> Vec<Tracer> {
        self.collectors
            .iter()
            .map(|(name, desc)| Tracer {
                name: name.clone(),
                description: desc.clone(),
            })
            .collect()
    }
}

/// Request body for starting a tracer on agent
#[derive(Debug, Serialize)]
pub struct TracerStartRequest {
    pub sink_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
}
