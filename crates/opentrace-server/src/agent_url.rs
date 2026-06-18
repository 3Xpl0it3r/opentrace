// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use crate::errors::AppError;

const DEFAULT_AGENT_PORT: u16 = 8000;

pub fn build_agent_url(host: &str, path: &str) -> Result<String, AppError> {
    let host = host.trim();
    if host.is_empty() {
        return Err(AppError::BadRequest("agent host is required".to_string()));
    }

    let base = if host.contains("://") {
        host.to_string()
    } else if host.matches(':').count() > 1 && !host.starts_with('[') {
        format!("http://[{host}]")
    } else {
        format!("http://{host}")
    };

    let mut url = reqwest::Url::parse(&base)
        .map_err(|e| AppError::BadRequest(format!("invalid agent host: {e}")))?;
    if url.port().is_none() {
        url.set_port(Some(DEFAULT_AGENT_PORT))
            .map_err(|_| AppError::BadRequest("invalid agent port".to_string()))?;
    }

    url.set_path(path.trim_start_matches('/'));
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_agent_url_adds_default_port() {
        let url = build_agent_url("agent.local", "/systeminfo").unwrap();

        assert_eq!(url, "http://agent.local:8000/systeminfo");
    }

    #[test]
    fn build_agent_url_preserves_explicit_port() {
        let url = build_agent_url("https://agent.local:9000/base", "/api/start/skbdrop").unwrap();

        assert_eq!(url, "https://agent.local:9000/api/start/skbdrop");
    }

    #[test]
    fn build_agent_url_supports_ipv6_hosts() {
        let url = build_agent_url("::1", "/systeminfo").unwrap();

        assert_eq!(url, "http://[::1]:8000/systeminfo");
    }
}
