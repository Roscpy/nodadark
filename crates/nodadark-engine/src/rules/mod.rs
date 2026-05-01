// nodadark-engine/src/rules/mod.rs
// Moteur de règles persistantes (fichier TOML)

use crate::ProxyConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RulesConfig {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Nom humain de la règle
    pub name: String,
    /// Si activée
    #[serde(default = "bool_true")]
    pub enabled: bool,
    /// Filtre de domaine (ex: "*.google.com", "example.com")
    pub domain: Option<String>,
    /// Filtre de chemin (ex: "/api/*")
    pub path: Option<String>,
    /// Filtre de méthode HTTP
    pub method: Option<String>,
    /// Action à effectuer
    pub action: RuleActionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleActionConfig {
    Drop,
    ModifyHeader { name: String, value: String },
    RemoveHeader { name: String },
    InjectHeader { name: String, value: String },
}

fn bool_true() -> bool {
    true
}

/// Action retournée après évaluation des règles
pub enum RuleAction {
    PassThrough,
    Drop,
    ModifyHeaders(Vec<(String, String)>),
}

pub struct RulesEngine {
    rules: Vec<Rule>,
}

impl RulesEngine {
    pub fn load_or_default(config: &Arc<ProxyConfig>) -> Self {
        let rules_path = Self::rules_path(config);
        let rules = if rules_path.exists() {
            match std::fs::read_to_string(&rules_path) {
                Ok(content) => match toml::from_str::<RulesConfig>(&content) {
                    Ok(cfg) => {
                        tracing::info!(
                            "✅ {} règle(s) chargée(s) depuis {}",
                            cfg.rules.len(),
                            rules_path.display()
                        );
                        cfg.rules
                    }
                    Err(e) => {
                        tracing::warn!("Erreur parsing rules.toml: {e}");
                        vec![]
                    }
                },
                Err(_) => vec![],
            }
        } else {
            // Créer un fichier d'exemple
            if let Some(parent) = rules_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let example = Self::example_config();
            let _ = std::fs::write(&rules_path, toml::to_string_pretty(&example).unwrap_or_default());
            tracing::info!("Fichier de règles créé : {}", rules_path.display());
            vec![]
        };

        Self { rules }
    }

    pub fn evaluate(
        &self,
        host: &str,
        path: &str,
        _headers: &[(String, String)],
    ) -> RuleAction {
        let mut modifications: Vec<(String, String)> = vec![];

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            if !self.matches(rule, host, path) {
                continue;
            }

            match &rule.action {
                RuleActionConfig::Drop => {
                    tracing::debug!("Règle '{}' → DROP pour {host}{path}", rule.name);
                    return RuleAction::Drop;
                }
                RuleActionConfig::ModifyHeader { name, value }
                | RuleActionConfig::InjectHeader { name, value } => {
                    modifications.push((name.clone(), value.clone()));
                }
                RuleActionConfig::RemoveHeader { name } => {
                    // Marquer pour suppression (valeur vide = suppression)
                    modifications.push((name.clone(), String::new()));
                }
            }
        }

        if modifications.is_empty() {
            RuleAction::PassThrough
        } else {
            RuleAction::ModifyHeaders(modifications)
        }
    }

    fn matches(&self, rule: &Rule, host: &str, path: &str) -> bool {
        if let Some(domain_pattern) = &rule.domain {
            if !glob_match(domain_pattern, host) {
                return false;
            }
        }
        if let Some(path_pattern) = &rule.path {
            if !glob_match(path_pattern, path) {
                return false;
            }
        }
        true
    }

    fn rules_path(config: &Arc<ProxyConfig>) -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nodadark")
            .join("rules.toml")
    }

    fn example_config() -> RulesConfig {
        RulesConfig {
            rules: vec![
                Rule {
                    name: "Exemple: supprimer X-Frame-Options".into(),
                    enabled: false,
                    domain: Some("*.example.com".into()),
                    path: None,
                    method: None,
                    action: RuleActionConfig::RemoveHeader {
                        name: "X-Frame-Options".into(),
                    },
                },
                Rule {
                    name: "Exemple: remplacer User-Agent".into(),
                    enabled: false,
                    domain: None,
                    path: None,
                    method: None,
                    action: RuleActionConfig::ModifyHeader {
                        name: "User-Agent".into(),
                        value: "NodaDark/0.1 SecurityAudit".into(),
                    },
                },
            ],
        }
    }
}

/// Correspondance glob simple (* = n'importe quoi)
fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return text == suffix || text.ends_with(&format!(".{suffix}"));
    }
    pattern == text
}
