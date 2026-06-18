use crate::{agent_url::build_agent_url, db::Database};
use std::sync::Arc;
use std::time::Duration;

const CHECK_INTERVAL: Duration = Duration::from_secs(60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Start the background health check task.
/// Every 30 seconds, queries each agent's /systeminfo endpoint
/// and updates their status to "online" or "offline".
pub fn start(db: Arc<Database>) {
    tokio::spawn(async move {
        eprintln!("[health] started, interval={:?}", CHECK_INTERVAL);
        loop {
            tokio::time::sleep(CHECK_INTERVAL).await;
            check_all_agents(&db).await;
        }
    });
}

async fn check_all_agents(db: &Arc<Database>) {
    let agents = match db.list_all_agents() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[health] failed to list agents: {}", e);
            return;
        }
    };

    if agents.is_empty() {
        return;
    }

    eprintln!("[health] checking {} agents...", agents.len());

    let client = match reqwest::Client::builder()
        .connect_timeout(REQUEST_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[health] failed to create HTTP client: {}", e);
            return;
        }
    };

    for agent in &agents {
        let url = match build_agent_url(&agent.host, "/systeminfo") {
            Ok(url) => url,
            Err(e) => {
                eprintln!("[health] agent {} has invalid host: {}", agent.name, e);
                if agent.status != "offline" {
                    let _ = db.update_agent_status(agent.id, "offline");
                }
                continue;
            }
        };

        let mut request = client.get(&url);
        if let Some(ref token) = agent.token {
            if !token.is_empty() {
                request = request.header("Authorization", format!("Bearer {}", token));
            }
        }

        let new_status = match request.send().await {
            Ok(resp) if resp.status().is_success() => "online",
            Ok(resp) => {
                eprintln!(
                    "[health] agent {} ({}) returned {}",
                    agent.name,
                    agent.host,
                    resp.status()
                );
                "offline"
            }
            Err(e) => {
                eprintln!(
                    "[health] agent {} ({}) unreachable: {}",
                    agent.name, agent.host, e
                );
                "offline"
            }
        };

        if agent.status != new_status {
            if let Err(e) = db.update_agent_status(agent.id, new_status) {
                eprintln!("[health] failed to update status for {}: {}", agent.name, e);
            } else {
                eprintln!(
                    "[health] agent {} status: {} -> {}",
                    agent.name, agent.status, new_status
                );
            }
        }

        // When agent is online, sync collector running state to tracepoints
        if new_status == "online" {
            let status_url = match build_agent_url(&agent.host, "/status") {
                Ok(url) => url,
                Err(_) => continue,
            };
            let mut status_request = client.get(&status_url);
            if let Some(ref token) = agent.token {
                if !token.is_empty() {
                    status_request = status_request.header("Authorization", format!("Bearer {}", token));
                }
            }
            match status_request.send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(body) = resp.text().await {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                            if let Some(items) = v["collectors"]["items"].as_array() {
                                // First: set ALL tracepoints to stopped
                                let _ = db.set_all_tracepoints_enabled(agent.id, false);
                                for item in items {
                                    if let (Some(name), Some(state)) = (
                                        item["name"].as_str(),
                                        item["state"].as_str(),
                                    ) {
                                        if state == "running" {
                                            let _ = db.set_tracepoint_enabled(agent.id, name, true);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
