#[derive(Debug, Clone, serde::Deserialize)]
pub struct MvtRsSettings {
    pub base_url: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Settings {
    pub mvt_rs: MvtRsSettings,
}

impl Settings {
    /// Test-only entry point: takes config content and env overrides as
    /// in-memory values (no real files, no real process env touched), so
    /// precedence tests are deterministic and parallel-safe.
    #[allow(dead_code)]
    fn load_from(
        yaml: Option<&str>,
        env_overrides: Option<std::collections::HashMap<String, String>>,
    ) -> Result<Self, config::ConfigError> {
        let mut builder = config::Config::builder();
        if let Some(yaml) = yaml {
            builder = builder.add_source(config::File::from_str(yaml, config::FileFormat::Yaml));
        }
        let mut env = config::Environment::with_prefix("MVT_SERVER_MCP")
            .prefix_separator("__")
            .separator("__")
            .try_parsing(true);
        if let Some(map) = env_overrides {
            env = env.source(Some(map.into_iter().collect()));
        }
        let settings: Settings = builder.add_source(env).build()?.try_deserialize()?;
        settings.validate().map_err(config::ConfigError::Message)?;
        Ok(settings)
    }

    fn validate(&self) -> Result<(), String> {
        if self.mvt_rs.base_url.trim().is_empty() {
            return Err(
                "mvt_rs.base_url must be set (config.yaml or MVT_SERVER_MCP__MVT_RS__BASE_URL)"
                    .into(),
            );
        }
        if !self.mvt_rs.base_url.starts_with("http://")
            && !self.mvt_rs.base_url.starts_with("https://")
        {
            return Err(format!(
                "mvt_rs.base_url must start with http:// or https:// (got '{}')",
                self.mvt_rs.base_url
            ));
        }
        if self.mvt_rs.email.trim().is_empty() {
            return Err(
                "mvt_rs.email must be set (config.yaml or MVT_SERVER_MCP__MVT_RS__EMAIL)".into(),
            );
        }
        if self.mvt_rs.password.trim().is_empty() {
            return Err(
                "mvt_rs.password must be set (config.yaml or MVT_SERVER_MCP__MVT_RS__PASSWORD)"
                    .into(),
            );
        }
        Ok(())
    }

    /// Production entry point: resolves the config file path from
    /// `MVT_SERVER_MCP_CONFIG_FILE` (default `"config.yaml"`), layers real env vars
    /// (`MVT_SERVER_MCP__...`) on top, validates, and returns.
    pub fn load() -> Result<Self, config::ConfigError> {
        let path = std::env::var("MVT_SERVER_MCP_CONFIG_FILE")
            .unwrap_or_else(|_| "config.yaml".to_string());
        let builder = config::Config::builder()
            .add_source(config::File::new(&path, config::FileFormat::Yaml).required(false))
            .add_source(
                config::Environment::with_prefix("MVT_SERVER_MCP")
                    .prefix_separator("__")
                    .separator("__")
                    .try_parsing(true),
            );
        let settings: Settings = builder.build()?.try_deserialize()?;
        settings.validate().map_err(config::ConfigError::Message)?;
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn missing_everything_is_a_config_error() {
        let result = Settings::load_from(None, Some(HashMap::new()));
        assert!(result.is_err());
    }

    #[test]
    fn yaml_file_alone_is_sufficient() {
        let yaml = r#"
mvt_rs:
  base_url: "http://localhost:8080"
  email: "test@example.com"
  password: "secret"
"#;
        let result = Settings::load_from(Some(yaml), Some(HashMap::new()));
        assert!(result.is_ok());
        let settings = result.unwrap();
        assert_eq!(settings.mvt_rs.base_url, "http://localhost:8080");
        assert_eq!(settings.mvt_rs.email, "test@example.com");
        assert_eq!(settings.mvt_rs.password, "secret");
    }

    #[test]
    fn env_var_overrides_yaml_value() {
        let yaml = r#"
mvt_rs:
  base_url: "http://a"
  email: "test@example.com"
  password: "secret"
"#;
        let mut env_overrides = HashMap::new();
        env_overrides.insert(
            "MVT_SERVER_MCP__MVT_RS__BASE_URL".to_string(),
            "http://b".to_string(),
        );

        let result = Settings::load_from(Some(yaml), Some(env_overrides));
        assert!(result.is_ok());
        let settings = result.unwrap();
        assert_eq!(settings.mvt_rs.base_url, "http://b");
        assert_eq!(settings.mvt_rs.email, "test@example.com");
        assert_eq!(settings.mvt_rs.password, "secret");
    }

    #[test]
    fn env_only_no_yaml_file() {
        let mut env_overrides = HashMap::new();
        env_overrides.insert(
            "MVT_SERVER_MCP__MVT_RS__BASE_URL".to_string(),
            "http://localhost:8080".to_string(),
        );
        env_overrides.insert(
            "MVT_SERVER_MCP__MVT_RS__EMAIL".to_string(),
            "test@example.com".to_string(),
        );
        env_overrides.insert(
            "MVT_SERVER_MCP__MVT_RS__PASSWORD".to_string(),
            "secret".to_string(),
        );

        let result = Settings::load_from(None, Some(env_overrides));
        assert!(result.is_ok());
        let settings = result.unwrap();
        assert_eq!(settings.mvt_rs.base_url, "http://localhost:8080");
        assert_eq!(settings.mvt_rs.email, "test@example.com");
        assert_eq!(settings.mvt_rs.password, "secret");
    }

    #[test]
    fn invalid_base_url_scheme_is_rejected() {
        let yaml = r#"
mvt_rs:
  base_url: "ftp://localhost:8080"
  email: "test@example.com"
  password: "secret"
"#;
        let result = Settings::load_from(Some(yaml), Some(HashMap::new()));
        assert!(result.is_err());
    }

    #[test]
    fn blank_password_is_rejected() {
        let yaml = r#"
mvt_rs:
  base_url: "http://localhost:8080"
  email: "test@example.com"
  password: ""
"#;
        let result = Settings::load_from(Some(yaml), Some(HashMap::new()));
        assert!(result.is_err());
    }
}
