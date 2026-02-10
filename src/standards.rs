use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct StandardsConfig {
    pub requirements: Vec<Check>,
    pub nice_to_haves: Vec<Check>,
    #[serde(default)]
    pub languages: HashMap<String, LanguageChecks>,
}

#[derive(Debug, Deserialize)]
pub struct LanguageChecks {
    #[serde(default)]
    pub requirements: Vec<Check>,
    #[serde(default)]
    pub nice_to_haves: Vec<Check>,
}

#[derive(Debug, Deserialize)]
pub struct Check {
    pub name: String,
    pub check: String,
}

impl StandardsConfig {
    pub fn from_str(input: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(input)
    }
}
