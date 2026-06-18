use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sink {
    pub id: i64,
    pub name: String,
    pub sink_type: String,
    pub config: String,
    pub status: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateSink {
    pub name: String,
    pub sink_type: String,
    pub config: String,
    /// Optional list of agent IDs to deploy this sink to.
    /// When provided, the server will call each agent's API to create the sink.
    pub agent_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSink {
    pub name: Option<String>,
    pub sink_type: Option<String>,
    pub config: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BindAgentRequest {
    pub agent_id: i64,
}

/// Response from sink deployment to agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkDeployResult {
    pub agent_id: i64,
    pub agent_name: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Response from creating a sink (includes optional deploy results)
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSinkResponse {
    pub id: i64,
    pub name: String,
    pub sink_type: String,
    pub config: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    /// Deployment results when agent_ids were provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy_results: Option<Vec<SinkDeployResult>>,
}
