use std::collections::HashSet;

use ratatui::style::Color;

use crate::provider::{Package, Provider, SearchResult};
use crate::util::{command_exists, escape_query, exec_command, sort_by_relevance};

pub struct FlatpakProvider;

fn get_installed() -> HashSet<String> {
    let output = exec_command("flatpak list --columns=application 2>/dev/null");
    output.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect()
}

impl Provider for FlatpakProvider {
    fn name(&self) -> &str {
        "flatpak"
    }

    fn is_available(&self) -> bool {
        command_exists("flatpak")
    }

    fn search(&self, query: &str) -> SearchResult {
        if query.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let escaped = escape_query(query);
        let output = exec_command(&format!("flatpak search '{escaped}' 2>/dev/null"));
        if output.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let installed = get_installed();
        let mut packages = Vec::new();
        let mut seen = HashSet::new();

        let mut lines = output.lines();
        lines.next(); // skip header row

        for line in lines {
            if line.is_empty() {
                continue;
            }
            // Output is tab-separated:
            // Name\tDescription\tApplication ID\tVersion\tBranch\tRemotes
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 3 {
                continue;
            }
            let description = cols[1].trim().to_string();
            let app_id = cols[2].trim().to_string();
            let version = if cols.len() > 3 { cols[3].trim().to_string() } else { String::new() };
            let remote = if cols.len() > 5 {
                cols[5].split_whitespace().next().unwrap_or("flathub").to_string()
            } else {
                "flathub".to_string()
            };

            if app_id.is_empty() || !seen.insert(app_id.clone()) {
                continue;
            }

            let is_installed = installed.contains(&app_id);
            packages.push(Package {
                name: app_id,
                version,
                description,
                source: remote,
                installed: is_installed,
            });
        }

        sort_by_relevance(&mut packages, query);
        SearchResult { packages, error: None }
    }

    fn install_command(&self, pkg: &Package) -> String {
        format!("flatpak install {} {}", pkg.source, pkg.name)
    }

    fn source_color(&self, _source: &str) -> Color {
        Color::Blue
    }
}
