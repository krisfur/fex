use std::collections::HashSet;

use ratatui::style::Color;

use crate::provider::{Package, Provider, SearchResult};
use crate::util::{command_exists, escape_query, exec_command, sort_by_relevance};

pub struct ZerobrewProvider;

fn get_installed() -> HashSet<String> {
    let mut installed = HashSet::new();
    for output in [
        exec_command("brew list --formula 2>/dev/null"),
        exec_command("brew list --cask 2>/dev/null"),
    ] {
        for line in output.lines() {
            if !line.is_empty() {
                installed.insert(line.to_string());
            }
        }
    }
    installed
}

impl Provider for ZerobrewProvider {
    fn name(&self) -> &str {
        "zerobrew"
    }

    fn is_available(&self) -> bool {
        command_exists("zerobrew")
    }

    fn search(&self, query: &str) -> SearchResult {
        if query.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let escaped = escape_query(query);
        let installed = get_installed();

        // Try exact match via brew info
        let mut exact_match: Option<Package> = None;
        let info_output = exec_command(&format!("brew info '{escaped}' 2>/dev/null"));
        if !info_output.is_empty() && !info_output.contains("Error:") {
            let mut lines = info_output.lines();
            if let Some(first_line) = lines.next() {
                let first_line = first_line.strip_prefix("==> ").unwrap_or(first_line);
                if let Some(colon) = first_line.find(": ") {
                    let name = first_line[..colon].to_string();
                    let description = lines
                        .find(|l| {
                            !l.is_empty()
                                && !l.starts_with('=')
                                && !l.starts_with("http")
                                && !l.starts_with("Installed")
                                && !l.starts_with("From:")
                                && !l.starts_with("License:")
                        })
                        .unwrap_or("")
                        .to_string();
                    let is_installed = installed.contains(&name);
                    exact_match = Some(Package {
                        name,
                        version: String::new(),
                        description,
                        source: "formula".to_string(),
                        installed: is_installed,
                    });
                }
            }
        }

        let output = exec_command(&format!("brew search --desc '{escaped}' 2>/dev/null"));

        let mut packages = Vec::new();

        let exact_name = exact_match.as_ref().map(|p| p.name.clone()).unwrap_or_default();
        if let Some(em) = exact_match {
            packages.push(em);
        }

        let mut current_source = "formula";
        for line in output.lines() {
            if line.is_empty() {
                continue;
            }
            if line.contains("==> Formulae") {
                current_source = "formula";
                continue;
            }
            if line.contains("==> Casks") {
                current_source = "cask";
                continue;
            }
            if line.starts_with('=') || line.starts_with("No ") {
                continue;
            }

            let (name, description) = if let Some(colon) = line.find(": ") {
                (line[..colon].to_string(), line[colon + 2..].to_string())
            } else {
                (line.trim().to_string(), String::new())
            };

            if name.is_empty() || name == exact_name {
                continue;
            }

            let is_installed = installed.contains(&name);
            packages.push(Package {
                name,
                version: String::new(),
                description,
                source: current_source.to_string(),
                installed: is_installed,
            });
        }

        sort_by_relevance(&mut packages, query);
        SearchResult { packages, error: None }
    }

    fn install_command(&self, pkg: &Package) -> String {
        if pkg.source == "cask" {
            format!("brew install --cask {}", pkg.name)
        } else {
            format!("brew install {}", pkg.name)
        }
    }

    fn source_color(&self, source: &str) -> Color {
        match source {
            "cask" => Color::Magenta,
            _ => Color::LightGreen,
        }
    }
}
