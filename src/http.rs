use anyhow::{Context, Result};

pub trait HttpClient {
    fn get_json(&self, url: &str) -> Result<serde_json::Value>;
}

pub struct UreqHttpClient;

impl HttpClient for UreqHttpClient {
    fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let mut req = ureq::get(url).header("User-Agent", "skilltree-cli");

        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            req = req.header("Authorization", &format!("Bearer {}", token));
        }

        let text = req
            .call()
            .with_context(|| format!("HTTP request failed: {}", url))?
            .body_mut()
            .read_to_string()
            .with_context(|| format!("failed to read response from {}", url))?;

        let body: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse JSON from {}", url))?;

        Ok(body)
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    pub struct MockHttpClient {
        responses: RefCell<VecDeque<Result<serde_json::Value>>>,
    }

    impl MockHttpClient {
        pub fn new(responses: Vec<serde_json::Value>) -> Self {
            Self {
                responses: RefCell::new(responses.into_iter().map(Ok).collect()),
            }
        }

        pub fn with_error(msg: &str) -> Self {
            let mut q = VecDeque::new();
            q.push_back(Err(anyhow::anyhow!("{}", msg)));
            Self {
                responses: RefCell::new(q),
            }
        }
    }

    impl HttpClient for MockHttpClient {
        fn get_json(&self, _url: &str) -> Result<serde_json::Value> {
            self.responses
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| anyhow::bail!("MockHttpClient: no more responses"))
        }
    }
}
