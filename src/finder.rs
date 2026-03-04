use anyhow::{Context, Result};

use crate::http::HttpClient;

pub struct FindOpts {
    pub query: String,
    pub limit: usize,
}

pub struct FoundSkill {
    pub full_name: String,
    pub description: String,
    pub stars: u64,
    pub url: String,
}

pub fn find_skills(opts: &FindOpts, http: &dyn HttpClient) -> Result<Vec<FoundSkill>> {
    let url = format!(
        "https://api.github.com/search/repositories?q={}+topic:agent-skill&sort=stars&per_page={}",
        opts.query, opts.limit
    );

    let json = http
        .get_json(&url)
        .context("failed to search GitHub for skills")?;

    let items = match json.get("items").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Ok(Vec::new()),
    };

    let skills = items
        .iter()
        .map(|item| FoundSkill {
            full_name: item["full_name"].as_str().unwrap_or("").to_string(),
            description: item["description"].as_str().unwrap_or("").to_string(),
            stars: item["stargazers_count"].as_u64().unwrap_or(0),
            url: item["html_url"].as_str().unwrap_or("").to_string(),
        })
        .collect();

    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::mock::MockHttpClient;
    use serde_json::json;

    #[test]
    fn parse_search_results() {
        let response = json!({
            "total_count": 2,
            "items": [
                {
                    "full_name": "user/skill-a",
                    "description": "A cool skill",
                    "stargazers_count": 42,
                    "html_url": "https://github.com/user/skill-a"
                },
                {
                    "full_name": "org/skill-b",
                    "description": "Another skill",
                    "stargazers_count": 7,
                    "html_url": "https://github.com/org/skill-b"
                }
            ]
        });

        let http = MockHttpClient::new(vec![response]);
        let opts = FindOpts {
            query: "test".into(),
            limit: 10,
        };

        let results = find_skills(&opts, &http).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].full_name, "user/skill-a");
        assert_eq!(results[0].description, "A cool skill");
        assert_eq!(results[0].stars, 42);
        assert_eq!(results[0].url, "https://github.com/user/skill-a");
        assert_eq!(results[1].full_name, "org/skill-b");
        assert_eq!(results[1].stars, 7);
    }

    #[test]
    fn parse_partial_item_fields() {
        let response = json!({
            "total_count": 1,
            "items": [
                {
                    "full_name": "user/minimal",
                    "stargazers_count": 0
                }
            ]
        });

        let http = MockHttpClient::new(vec![response]);
        let opts = FindOpts {
            query: "q".into(),
            limit: 5,
        };

        let results = find_skills(&opts, &http).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].full_name, "user/minimal");
        assert_eq!(results[0].description, "");
        assert_eq!(results[0].url, "");
        assert_eq!(results[0].stars, 0);
    }

    #[test]
    fn empty_results() {
        let response = json!({ "total_count": 0, "items": [] });
        let http = MockHttpClient::new(vec![response]);
        let opts = FindOpts {
            query: "nonexistent".into(),
            limit: 10,
        };

        let results = find_skills(&opts, &http).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn missing_items_field() {
        let response = json!({ "total_count": 0 });
        let http = MockHttpClient::new(vec![response]);
        let opts = FindOpts {
            query: "q".into(),
            limit: 5,
        };

        let results = find_skills(&opts, &http).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn http_error_propagates() {
        let http = MockHttpClient::with_error("network failure");
        let opts = FindOpts {
            query: "q".into(),
            limit: 5,
        };

        assert!(find_skills(&opts, &http).is_err());
    }
}
