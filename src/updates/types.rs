pub trait Filterable {
    fn name(&self) -> &str;
}

pub fn filter_items<'a, T: Filterable>(items: &'a [T], query: &str) -> Vec<(usize, &'a T)> {
    if query.is_empty() {
        items.iter().enumerate().collect()
    } else {
        let query_lower = query.to_lowercase();
        items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.name().to_lowercase().contains(&query_lower))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageSource {
    Pacman,
    Aur,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub old_version: String,
    pub new_version: String,
    pub source: PackageSource,
    pub selected: bool,
}

impl Package {
    pub fn new(name: String, old_version: String, new_version: String, source: PackageSource) -> Self {
        Self {
            name,
            old_version,
            new_version,
            source,
            selected: false,
        }
    }

    pub fn source_label(&self) -> &'static str {
        match self.source {
            PackageSource::Pacman => "",
            PackageSource::Aur => " (AUR)",
        }
    }
}

impl Filterable for Package {
    fn name(&self) -> &str {
        &self.name
    }
}
