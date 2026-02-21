use ratatui::style::Color;

use crate::provider::{Package, Provider, SearchResult};
use crate::util::{command_exists, escape_query, exec_command, sort_by_relevance};

pub struct ZypperProvider;

fn field_value(line: &str, key: &str) -> Option<String> {
    if line.starts_with(key) {
        if let Some(colon) = line.find(':') {
            return Some(line[colon + 1..].trim().to_string());
        }
    }
    None
}

impl Provider for ZypperProvider {
    fn name(&self) -> &str {
        "zypper"
    }

    fn is_available(&self) -> bool {
        command_exists("zypper")
    }

    fn search(&self, query: &str) -> SearchResult {
        if query.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let escaped = escape_query(query);

        // Try exact match via zypper info
        let info_output =
            exec_command(&format!("zypper --quiet info '{escaped}' 2>/dev/null"));
        let mut exact_match: Option<Package> = None;
        if !info_output.is_empty() && !info_output.contains("not found") {
            let mut pkg = Package {
                name: String::new(),
                version: String::new(),
                description: String::new(),
                source: String::new(),
                installed: false,
            };
            for line in info_output.lines() {
                if let Some(val) = field_value(line, "Repository") {
                    pkg.source = val;
                } else if let Some(val) = field_value(line, "Name") {
                    pkg.name = val;
                } else if let Some(val) = field_value(line, "Version") {
                    pkg.version = val;
                } else if let Some(val) = field_value(line, "Summary") {
                    pkg.description = val;
                } else if let Some(val) = field_value(line, "Installed") {
                    pkg.installed = val.contains("Yes");
                }
            }
            if !pkg.name.is_empty() {
                exact_match = Some(pkg);
            }
        }

        let output =
            exec_command(&format!("zypper --quiet search '{escaped}' 2>/dev/null"));
        if output.is_empty() && exact_match.is_none() {
            return SearchResult { packages: vec![], error: None };
        }

        let mut packages = Vec::new();

        for line in output.lines() {
            if line.is_empty() {
                continue;
            }
            // Skip header and separator lines
            if line.contains("Name") && line.contains("Summary") {
                continue;
            }
            if line.starts_with("---") || line.contains("--+") {
                continue;
            }

            // Parse pipe-delimited: "S | Name | Summary | Type"
            let fields: Vec<&str> = line.split('|').map(|f| f.trim()).collect();
            if fields.len() < 3 {
                continue;
            }

            let name = fields[1].to_string();
            if name.is_empty() {
                continue;
            }

            let is_installed = fields[0] == "i" || fields[0] == "i+";
            let description = fields[2].to_string();
            let source = if fields.len() > 3 {
                fields[3].to_string()
            } else {
                "zypper".to_string()
            };

            packages.push(Package {
                name,
                version: String::new(),
                description,
                source,
                installed: is_installed,
            });
        }

        if let Some(em) = exact_match {
            let already = packages.iter().any(|p| p.name == em.name);
            if !already {
                packages.insert(0, em);
            }
        }

        sort_by_relevance(&mut packages, query);
        SearchResult { packages, error: None }
    }

    fn install_command(&self, pkg: &Package) -> String {
        format!("sudo zypper install {}", pkg.name)
    }

    fn source_color(&self, source: &str) -> Color {
        match source {
            "repo-oss" => Color::Green,
            "repo-non-oss" => Color::Yellow,
            "repo-update" => Color::Blue,
            "repo-update-non-oss" => Color::Magenta,
            _ => Color::Cyan,
        }
    }
}
