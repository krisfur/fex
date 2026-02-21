use std::collections::HashSet;

use ratatui::style::Color;

use crate::provider::{Package, Provider, SearchResult};
use crate::util::{command_exists, escape_query, exec_command, sort_by_relevance};

pub struct AptProvider;

fn get_installed() -> HashSet<String> {
    let output = exec_command("dpkg-query -W -f='${Package}\\n' 2>/dev/null");
    output.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect()
}

impl Provider for AptProvider {
    fn name(&self) -> &str {
        "apt"
    }

    fn is_available(&self) -> bool {
        command_exists("apt-cache")
    }

    fn search(&self, query: &str) -> SearchResult {
        if query.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let escaped = escape_query(query);
        let output = exec_command(&format!("apt-cache search '{escaped}' 2>/dev/null"));
        if output.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let installed = get_installed();
        let mut packages = Vec::new();

        for line in output.lines() {
            if line.is_empty() {
                continue;
            }
            // Format: package-name - description
            let Some(sep) = line.find(" - ") else { continue };
            let name = line[..sep].to_string();
            let description = line[sep + 3..].to_string();
            let is_installed = installed.contains(&name);
            packages.push(Package {
                name,
                version: String::new(),
                description,
                source: "apt".to_string(),
                installed: is_installed,
            });
        }

        sort_by_relevance(&mut packages, query);
        SearchResult { packages, error: None }
    }

    fn install_command(&self, pkg: &Package) -> String {
        format!("sudo apt install {}", pkg.name)
    }

    fn source_color(&self, _source: &str) -> Color {
        Color::Yellow
    }
}
