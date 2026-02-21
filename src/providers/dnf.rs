use std::collections::HashSet;

use ratatui::style::Color;

use crate::provider::{Package, Provider, SearchResult};
use crate::util::{command_exists, escape_query, exec_command, sort_by_relevance};

pub struct DnfProvider;

fn get_installed() -> HashSet<String> {
    let output = exec_command("rpm -qa --qf '%{NAME}\\n' 2>/dev/null");
    output.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect()
}

impl Provider for DnfProvider {
    fn name(&self) -> &str {
        "dnf"
    }

    fn is_available(&self) -> bool {
        command_exists("dnf")
    }

    fn search(&self, query: &str) -> SearchResult {
        if query.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let escaped = escape_query(query);
        let output = exec_command(&format!("dnf search '{escaped}' 2>/dev/null"));
        if output.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let installed = get_installed();
        let mut packages = Vec::new();

        for line in output.lines() {
            if line.is_empty() {
                continue;
            }
            // Lines start with a space
            if !line.starts_with(' ') {
                continue;
            }
            if line.contains("Matched fields:")
                || line.contains("Updating")
                || line.contains("Repositories")
            {
                continue;
            }

            let line = line.trim_start();

            // Format: "name.arch   description"
            let Some(dot) = line.find('.') else { continue };
            let Some(arch_end) = line.find(' ') else { continue };
            if arch_end <= dot {
                continue;
            }
            let name_arch = &line[..arch_end];
            let description = line[arch_end..].trim_start().to_string();

            // Strip .arch suffix
            let name = match name_arch.rfind('.') {
                Some(last_dot) => name_arch[..last_dot].to_string(),
                None => name_arch.to_string(),
            };

            let is_installed = installed.contains(&name);
            packages.push(Package {
                name,
                version: String::new(),
                description,
                source: "fedora".to_string(),
                installed: is_installed,
            });
        }

        sort_by_relevance(&mut packages, query);
        SearchResult { packages, error: None }
    }

    fn install_command(&self, pkg: &Package) -> String {
        format!("sudo dnf install {}", pkg.name)
    }

    fn source_color(&self, source: &str) -> Color {
        match source {
            "fedora" => Color::Blue,
            "updates" => Color::Green,
            "@System" => Color::Cyan,
            _ => Color::Yellow,
        }
    }
}
