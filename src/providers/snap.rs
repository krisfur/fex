use std::collections::HashSet;

use ratatui::style::Color;

use crate::provider::{Package, Provider, SearchResult};
use crate::util::{command_exists, escape_query, exec_command, sort_by_relevance};

pub struct SnapProvider;

fn get_installed() -> HashSet<String> {
    let output = exec_command("snap list 2>/dev/null");
    let mut installed = HashSet::new();
    let mut lines = output.lines();
    lines.next(); // skip header
    for line in lines {
        if let Some(name) = line.split_whitespace().next() {
            installed.insert(name.to_string());
        }
    }
    installed
}

impl Provider for SnapProvider {
    fn name(&self) -> &str {
        "snap"
    }

    fn is_available(&self) -> bool {
        command_exists("snap")
    }

    fn search(&self, query: &str) -> SearchResult {
        if query.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let escaped = escape_query(query);
        let output = exec_command(&format!("snap find '{escaped}' 2>/dev/null"));
        if output.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let installed = get_installed();
        let mut packages = Vec::new();

        let mut lines = output.lines();
        lines.next(); // skip header row

        for line in lines {
            if line.is_empty() {
                continue;
            }
            // snap find output is space-padded; split on 2+ consecutive spaces
            let cols: Vec<&str> = line.split("  ").filter(|s| !s.is_empty()).collect();
            if cols.len() < 2 {
                continue;
            }
            let name = cols[0].trim().to_string();
            let version = if cols.len() > 1 { cols[1].trim().to_string() } else { String::new() };
            let description = if cols.len() > 4 {
                cols[4..].join("  ").trim().to_string()
            } else if cols.len() > 3 {
                cols[3].trim().to_string()
            } else {
                String::new()
            };

            if name.is_empty() {
                continue;
            }

            let is_installed = installed.contains(&name);
            packages.push(Package {
                name,
                version,
                description,
                source: "snap".to_string(),
                installed: is_installed,
            });
        }

        sort_by_relevance(&mut packages, query);
        SearchResult { packages, error: None }
    }

    fn install_command(&self, pkg: &Package) -> String {
        format!("sudo snap install {}", pkg.name)
    }

    fn source_color(&self, _source: &str) -> Color {
        Color::Yellow
    }
}
