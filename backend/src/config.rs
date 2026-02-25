use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub authentik_client_id: String,
    pub authentik_client_secret: String,
    pub authentik_base_url: String,
    pub app_base_url: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let jwt_secret = require_env("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 bytes".into());
        }

        Ok(Config {
            database_url: require_env("DATABASE_URL")?,
            jwt_secret,
            authentik_client_id: require_env("AUTHENTIK_CLIENT_ID")?,
            authentik_client_secret: require_env("AUTHENTIK_CLIENT_SECRET")?,
            authentik_base_url: require_env("AUTHENTIK_BASE_URL")?,
            app_base_url: require_env("APP_BASE_URL")?,
            smtp_host: require_env("SMTP_HOST")?,
            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".into())
                .parse()
                .map_err(|_| "SMTP_PORT must be a number".to_string())?,
            smtp_username: require_env("SMTP_USERNAME")?,
            smtp_password: require_env("SMTP_PASSWORD")?,
            smtp_from: require_env("SMTP_FROM")?,
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .map_err(|_| "PORT must be a number".to_string())?,
        })
    }
}

fn require_env(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("Missing required env var: {key}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_require_env_missing() {
        let result = require_env("DEFINITELY_NOT_SET_12345");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("DEFINITELY_NOT_SET_12345"));
    }
}
