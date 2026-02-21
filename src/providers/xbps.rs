use ratatui::style::Color;

use crate::provider::{Package, Provider, SearchResult};
use crate::util::{command_exists, escape_query, exec_command, sort_by_relevance};

pub struct XbpsProvider;

/// Split "name-version" by the last hyphen.
fn split_name_version(s: &str) -> (&str, &str) {
    match s.rfind('-') {
        Some(pos) if pos > 0 => (&s[..pos], &s[pos + 1..]),
        _ => (s, ""),
    }
}

impl Provider for XbpsProvider {
    fn name(&self) -> &str {
        "xbps"
    }

    fn is_available(&self) -> bool {
        command_exists("xbps-query")
    }

    fn search(&self, query: &str) -> SearchResult {
        if query.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let escaped = escape_query(query);
        let output = exec_command(&format!("xbps-query -Rs '{escaped}' 2>/dev/null"));
        if output.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let mut packages = Vec::new();

        for line in output.lines() {
            if line.len() < 5 {
                continue;
            }

            let installed = if line.starts_with("[*]") {
                true
            } else if line.starts_with("[-]") {
                false
            } else {
                continue;
            };

            // Skip "[*] " or "[-] " prefix (4 chars)
            let rest = &line[4..];

            // Find description (after two consecutive spaces)
            let (name_version, description) = match rest.find("  ") {
                Some(sep) => {
                    let nv = rest[..sep].trim_end();
                    let desc = rest[sep..].trim_start();
                    (nv, desc.to_string())
                }
                None => (rest.trim_end(), String::new()),
            };

            let (name, version) = split_name_version(name_version);
            packages.push(Package {
                name: name.to_string(),
                version: version.to_string(),
                description,
                source: "void".to_string(),
                installed,
            });
        }

        sort_by_relevance(&mut packages, query);
        SearchResult { packages, error: None }
    }

    fn install_command(&self, pkg: &Package) -> String {
        format!("sudo xbps-install {}", pkg.name)
    }

    fn source_color(&self, _source: &str) -> Color {
        Color::Green
    }
}
