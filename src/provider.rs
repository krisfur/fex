use ratatui::style::Color;

pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: String,
    pub installed: bool,
}

pub struct SearchResult {
    pub packages: Vec<Package>,
    pub error: Option<String>,
}

pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    fn search(&self, query: &str) -> SearchResult;
    fn install_command(&self, pkg: &Package) -> String;

    fn source_color(&self, source: &str) -> Color {
        match source {
            "core" => Color::Cyan,
            "extra" => Color::Green,
            "community" => Color::Yellow,
            "multilib" => Color::Magenta,
            "aur" => Color::LightBlue,
            _ => Color::White,
        }
    }
}

pub type BoxedProvider = Box<dyn Provider>;
