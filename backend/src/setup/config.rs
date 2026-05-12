use std::str::FromStr;

use worker::Env;

use crate::setup::exceptions::SetupError;

trait FromEnv: Sized {
    fn from_env(env: &Env) -> Result<Self, SetupError>;
}

#[derive(Clone)]
pub struct Config {
    pub security: SecurityConfig,
    pub ai: AiConfig,
}

#[derive(Clone)]
pub struct SecurityConfig {
    pub allowed_origins: Vec<String>,
}

impl FromEnv for SecurityConfig {
    fn from_env(env: &Env) -> Result<Self, SetupError> {
        Ok(Self {
            allowed_origins: Config::parse_csv(env, "ALLOWED_ORIGINS")?,
        })
    }
}

#[derive(Clone)]
pub struct AiConfig {
    pub inference: InferenceConfig,
}

#[derive(Clone)]
pub enum InferenceConfig {
    Cloudflare {
        generation_model: String,
    },

    #[cfg(feature = "ollama")]
    Ollama {
        url: String,
        generation_model: String,
    },
}

impl FromEnv for AiConfig {
    fn from_env(env: &Env) -> Result<Self, SetupError> {
        let generation_model = Config::parse(env, "GENERATION_MODEL")?;
        Ok(Self {
            inference: InferenceConfig::from_env(env, generation_model)?,
        })
    }
}

impl InferenceConfig {
    #[cfg(not(feature = "ollama"))]
    fn from_env(_env: &Env, generation_model: String) -> Result<Self, SetupError> {
        Ok(Self::Cloudflare { generation_model })
    }

    #[cfg(feature = "ollama")]
    fn from_env(env: &Env, generation_model: String) -> Result<Self, SetupError> {
        let provider = env
            .var("AI_PROVIDER")
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "cloudflare".to_string());

        match provider.as_str() {
            "cloudflare" => Ok(Self::Cloudflare { generation_model }),
            "ollama" => {
                let url = Config::parse(env, "OLLAMA_HOST")?;
                Ok(Self::Ollama {
                    url,
                    generation_model,
                })
            }
            other => Err(SetupError::InvalidVariable(format!(
                "AI_PROVIDER should be cloudflare or ollama, got {other}"
            ))),
        }
    }
}

impl Config {
    pub fn from_env(env: &Env) -> Result<Self, SetupError> {
        Ok(Config {
            security: SecurityConfig::from_env(env)?,
            ai: AiConfig::from_env(env)?,
        })
    }

    fn parse<T: FromStr>(env: &Env, var: &str) -> Result<T, SetupError> {
        let type_name = std::any::type_name::<T>();
        env.var(var)
            .map_err(|e| SetupError::MissingVariable(e.to_string()))?
            .to_string()
            .parse()
            .map_err(|_| SetupError::InvalidVariable(format!("{var} should be {type_name}")))
    }

    fn parse_csv(env: &Env, var: &str) -> Result<Vec<String>, SetupError> {
        Ok(env
            .var(var)
            .map_err(|_| SetupError::MissingVariable(var.to_string()))?
            .to_string()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }
}
