use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::Result;

pub struct BrokenRef {
    pub skill_name: String,
    pub referenced_path: String,
}

const INLINE_DIR_PREFIXES: &[&str] = &["references/", "scripts/", "assets/", "agents/"];

/// Extract internal file references from SKILL.md content (deduplicated).
pub fn extract_refs(content: &str) -> Vec<String> {
    let mut refs = BTreeSet::new();
    let mut in_code_block = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        for path in extract_markdown_refs(line) {
            refs.insert(path);
        }

        for token in line.split_whitespace() {
            if token.contains("](") {
                continue;
            }

            if let Some(path) = normalize_ref(token, false) {
                refs.insert(path);
            }
        }
    }

    refs.into_iter().collect()
}

fn extract_markdown_refs(line: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut search_from = 0;

    while let Some(start) = line[search_from..].find("](") {
        let path_start = search_from + start + 2;
        let Some(path_end) = line[path_start..].find(')') else {
            break;
        };

        let raw_path = &line[path_start..path_start + path_end];
        if let Some(path) = normalize_ref(raw_path, true) {
            refs.push(path);
        }

        search_from = path_start + path_end + 1;
    }

    refs
}

fn normalize_ref(raw: &str, is_markdown_link: bool) -> Option<String> {
    let trimmed = raw
        .trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | '(' | '[' | '<' | '*' | '_'))
        .trim_end_matches(|c: char| {
            matches!(
                c,
                '.' | ':' | ';' | ',' | '`' | '"' | '\'' | ')' | ']' | '>' | '*' | '_'
            )
        });

    let without_fragment = trimmed.split(['#', '?']).next().unwrap_or("");
    let normalized = without_fragment.trim_start_matches("./");

    if normalized.is_empty()
        || normalized.starts_with('#')
        || normalized.starts_with('/')
        || normalized.starts_with("../")
        || normalized.ends_with('/')
        || normalized.contains("://")
        || matches!(normalized.split_once(':'), Some((scheme, _)) if !scheme.contains('/'))
    {
        return None;
    }

    if !is_path_like(normalized, is_markdown_link) {
        return None;
    }

    Some(normalized.to_string())
}

fn is_path_like(path: &str, is_markdown_link: bool) -> bool {
    if !path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-'))
    {
        return false;
    }

    if path.contains('/') {
        let valid_segments = path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..");
        if !valid_segments {
            return false;
        }

        // Only check paths under known skill-internal directories.
        // Paths like `src/thahmm/entities/Book.ts` are example code paths,
        // not files that should exist inside the skill directory.
        return INLINE_DIR_PREFIXES
            .iter()
            .any(|prefix| path.starts_with(prefix));
    }

    // Sibling file (no slash): only detect in markdown links,
    // and must look like an actual filename (not `e.g`, `Next.js`, `0.84.0`)
    is_markdown_link && has_file_extension(path)
}

/// Check if a string looks like a filename with a proper extension.
/// Filters out version numbers (`0.84.0`), abbreviations (`e.g`),
/// framework names (`Next.js`), and bare extensions (`.html`).
fn has_file_extension(path: &str) -> bool {
    if path.starts_with('.') {
        return false;
    }

    let Some((base, ext)) = path.rsplit_once('.') else {
        return false;
    };

    // Base must be at least 2 chars and not purely numeric/dots
    if base.len() < 2 || base.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return false;
    }

    // Extension must be 1-5 alphabetic chars
    !ext.is_empty() && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphabetic())
}

/// Validate references in a single skill directory.
pub fn validate_skill_refs(skill_dir: &Path, skill_name: &str) -> Result<Vec<BrokenRef>> {
    let skill_md = skill_dir.join("SKILL.md");
    let content = match fs::read_to_string(&skill_md) {
        Ok(c) => c,
        Err(_) => return Ok(Vec::new()),
    };

    let refs = extract_refs(&content);
    let mut broken = Vec::new();

    for ref_path in refs {
        if !skill_dir.join(&ref_path).exists() {
            broken.push(BrokenRef {
                skill_name: skill_name.to_string(),
                referenced_path: ref_path,
            });
        }
    }

    Ok(broken)
}

/// Validate references across all skills in ~/.skilltree/.
pub fn validate_all_refs(skill_tree_dir: &Path) -> Result<Vec<BrokenRef>> {
    let mut all_broken = Vec::new();

    let entries = match fs::read_dir(skill_tree_dir) {
        Ok(e) => e,
        Err(_) => return Ok(all_broken),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let skill_name = name.to_string_lossy();
        if skill_name.starts_with('.') {
            continue;
        }

        let broken = validate_skill_refs(&path, &skill_name)?;
        all_broken.extend(broken);
    }

    Ok(all_broken)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn extract_refs_basic() {
        let content = "See `references/flow-patterns.md` for details.\n\
                       Run `scripts/search.py` to search.";
        let refs = extract_refs(content);
        assert_eq!(
            refs,
            vec!["references/flow-patterns.md", "scripts/search.py"]
        );
    }

    #[test]
    fn extract_refs_skips_code_blocks() {
        let content = "Before block.\n\
                       ```\n\
                       references/inside-code.md\n\
                       ```\n\
                       After: references/outside.md";
        let refs = extract_refs(content);
        assert_eq!(refs, vec!["references/outside.md"]);
    }

    #[test]
    fn extract_refs_skips_cross_skill() {
        let content = "See /other-skill/references/foo.md for cross-ref.\n\
                       But references/local.md is local.";
        let refs = extract_refs(content);
        assert_eq!(refs, vec!["references/local.md"]);
    }

    #[test]
    fn extract_refs_deduplicates() {
        let content = "references/a.md and references/a.md again";
        let refs = extract_refs(content);
        assert_eq!(refs, vec!["references/a.md"]);
    }

    #[test]
    fn extract_refs_directory_mentions_skipped() {
        let content = "The references/ directory contains files and we say and/or often.";
        let refs = extract_refs(content);
        assert!(refs.is_empty());
    }

    #[test]
    fn extract_refs_markdown_link() {
        let content = "See [patterns](references/flow-patterns.md) for details.";
        let refs = extract_refs(content);
        assert_eq!(refs, vec!["references/flow-patterns.md"]);
    }

    #[test]
    fn extract_refs_dot_slash_paths() {
        let content = "See [guide](./references/guide.md) and run ./scripts/search.py.";
        let refs = extract_refs(content);
        assert_eq!(refs, vec!["references/guide.md", "scripts/search.py"]);
    }

    #[test]
    fn extract_refs_strips_anchor() {
        let content = "See [details](references/guide.md#workflow) for more.";
        let refs = extract_refs(content);
        assert_eq!(refs, vec!["references/guide.md"]);
    }

    #[test]
    fn extract_refs_detects_sibling_and_asset_links() {
        let content =
            "See [forms](FORMS.md), [agent](agents/openai.yaml), and ![hero](assets/hero.png).";
        let refs = extract_refs(content);
        assert_eq!(
            refs,
            vec!["FORMS.md", "agents/openai.yaml", "assets/hero.png"]
        );
    }

    #[test]
    fn extract_refs_keeps_extensionless_internal_paths() {
        let content = "Run `scripts/bootstrap` and see [readme](references/README).";
        let refs = extract_refs(content);
        assert_eq!(refs, vec!["references/README", "scripts/bootstrap"]);
    }

    #[test]
    fn extract_refs_bold_wrapped() {
        let content = "Check **references/important.md** now.";
        let refs = extract_refs(content);
        assert_eq!(refs, vec!["references/important.md"]);
    }

    #[test]
    fn extract_refs_trailing_punctuation() {
        let content = "See references/foo.md.";
        let refs = extract_refs(content);
        assert_eq!(refs, vec!["references/foo.md"]);
    }

    #[test]
    fn extract_refs_ignores_external_links() {
        let content =
            "Visit [docs](https://example.com/guide.md) or [mail](mailto:test@example.com).";
        let refs = extract_refs(content);
        assert!(refs.is_empty());
    }

    #[test]
    fn validate_skill_refs_finds_broken() {
        let tmp = TempDir::new().unwrap();
        let skill = tmp.path().join("my-skill");
        fs::create_dir_all(skill.join("references")).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "Use references/exists.md and scripts/missing.py",
        )
        .unwrap();
        fs::write(skill.join("references/exists.md"), "ok").unwrap();

        let broken = validate_skill_refs(&skill, "my-skill").unwrap();
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].referenced_path, "scripts/missing.py");
        assert_eq!(broken[0].skill_name, "my-skill");
    }

    #[test]
    fn validate_skill_refs_handles_dot_slash_and_anchor() {
        let tmp = TempDir::new().unwrap();
        let skill = tmp.path().join("my-skill");
        fs::create_dir_all(skill.join("references")).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "See [guide](./references/guide.md#workflow) and run ./scripts/missing.py",
        )
        .unwrap();
        fs::write(skill.join("references/guide.md"), "ok").unwrap();

        let broken = validate_skill_refs(&skill, "my-skill").unwrap();
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].referenced_path, "scripts/missing.py");
    }

    #[test]
    fn validate_skill_refs_detects_sibling_asset_and_extensionless_broken_refs() {
        let tmp = TempDir::new().unwrap();
        let skill = tmp.path().join("my-skill");
        fs::create_dir_all(skill.join("assets")).unwrap();
        fs::create_dir_all(skill.join("references")).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "See [forms](FORMS.md), [hero](assets/hero.png), `scripts/bootstrap`, and [readme](references/README).",
        )
        .unwrap();
        fs::write(skill.join("FORMS.md"), "ok").unwrap();

        let broken = validate_skill_refs(&skill, "my-skill").unwrap();
        assert_eq!(broken.len(), 3);
        assert_eq!(
            broken
                .iter()
                .map(|broken| broken.referenced_path.as_str())
                .collect::<Vec<_>>(),
            vec!["assets/hero.png", "references/README", "scripts/bootstrap"]
        );
    }

    #[test]
    fn validate_skill_refs_no_skill_md() {
        let tmp = TempDir::new().unwrap();
        let skill = tmp.path().join("empty-skill");
        fs::create_dir_all(&skill).unwrap();

        let broken = validate_skill_refs(&skill, "empty-skill").unwrap();
        assert!(broken.is_empty());
    }

    #[test]
    fn validate_all_refs_scans_all_skills() {
        let tmp = TempDir::new().unwrap();

        // Skill with broken ref
        let s1 = tmp.path().join("skill-a");
        fs::create_dir_all(&s1).unwrap();
        fs::write(s1.join("SKILL.md"), "scripts/missing.sh").unwrap();

        // Skill with valid ref
        let s2 = tmp.path().join("skill-b");
        fs::create_dir_all(s2.join("references")).unwrap();
        fs::write(s2.join("SKILL.md"), "references/valid.md").unwrap();
        fs::write(s2.join("references/valid.md"), "ok").unwrap();

        let broken = validate_all_refs(tmp.path()).unwrap();
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].skill_name, "skill-a");
    }

    #[test]
    fn extract_refs_ignores_non_skill_paths() {
        // Version numbers, abbreviations, framework names, extensions,
        // code identifiers, and example project paths should NOT be detected
        let content = "\
            Use Next.js and Node.js for the frontend.\n\
            Version 0.84.0 is required, see section 1.3.4.\n\
            e.g. use flow.map or viewModelScope.launch for async.\n\
            Supports .html, .tsx, and .svelte files.\n\
            See src/thahmm/entities/Book.ts for the entity.\n\
            Edit package.json and metro.config.js as needed.\n\
            Check [guide](src/app/page.tsx) for examples.\n\
            Use MaterialTheme.colorScheme.primary for theming.\n\
        ";
        let refs = extract_refs(content);
        assert!(refs.is_empty(), "Expected no refs, got: {:?}", refs);
    }

    #[test]
    fn extract_refs_still_detects_real_skill_refs() {
        let content = "\
            See references/guide.md for details.\n\
            Run `scripts/deploy.sh` to deploy.\n\
            Check [forms](FORMS.md) and [logo](assets/logo.png).\n\
            Agent config: agents/default.yaml\n\
        ";
        let refs = extract_refs(content);
        assert_eq!(
            refs,
            vec![
                "FORMS.md",
                "agents/default.yaml",
                "assets/logo.png",
                "references/guide.md",
                "scripts/deploy.sh",
            ]
        );
    }
}
