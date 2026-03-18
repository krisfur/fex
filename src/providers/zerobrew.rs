use ratatui::style::Color;

use crate::provider::{Package, Provider, SearchResult};
use crate::providers::homebrew_index;
use crate::util::command_exists;

pub struct ZerobrewProvider;

impl Provider for ZerobrewProvider {
    fn name(&self) -> &str {
        "zerobrew"
    }

    fn is_available(&self) -> bool {
        command_exists("zerobrew")
    }

    fn search(&self, query: &str) -> SearchResult {
        homebrew_index::search(query)
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
