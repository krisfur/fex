use std::collections::HashSet;

use ratatui::style::Color;

use crate::provider::{Package, Provider, SearchResult};
use crate::util::{command_exists, escape_query, exec_command, sort_by_relevance};

pub struct ApkProvider;

fn get_installed() -> HashSet<String> {
    let output = exec_command("apk info 2>/dev/null");
    output.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect()
}

/// Split "name-version" by the last hyphen followed by a digit.
fn split_name_version(s: &str) -> (&str, &str) {
    let bytes = s.as_bytes();
    let mut i = s.len();
    while i > 0 {
        i -= 1;
        if bytes[i] == b'-' && i + 1 < s.len() && bytes[i + 1].is_ascii_digit() {
            return (&s[..i], &s[i + 1..]);
        }
    }
    (s, "")
}

impl Provider for ApkProvider {
    fn name(&self) -> &str {
        "apk"
    }

    fn is_available(&self) -> bool {
        command_exists("apk")
    }

    fn search(&self, query: &str) -> SearchResult {
        if query.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let escaped = escape_query(query);
        let output = exec_command(&format!("apk search -v '{escaped}' 2>/dev/null"));
        if output.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let installed = get_installed();
        let mut packages = Vec::new();

        for line in output.lines() {
            if line.is_empty() {
                continue;
            }
            // Format: package-name-version - description
            let Some(sep) = line.find(" - ") else { continue };
            let name_version = &line[..sep];
            let description = line[sep + 3..].to_string();

            let (name, version) = split_name_version(name_version);
            let is_installed = installed.contains(name);
            packages.push(Package {
                name: name.to_string(),
                version: version.to_string(),
                description,
                source: "alpine".to_string(),
                installed: is_installed,
            });
        }

        sort_by_relevance(&mut packages, query);
        SearchResult { packages, error: None }
    }

    fn install_command(&self, pkg: &Package) -> String {
        format!("sudo apk add {}", pkg.name)
    }

    fn source_color(&self, source: &str) -> Color {
        match source {
            "community" => Color::Yellow,
            _ => Color::Blue,
        }
    }
}
