use std::env::var;

use anyhow::Result;

pub struct AppConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub auth: AuthConfig,
}

#[derive(Debug)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
}

impl AppConfig {
    pub fn new() -> Result<Self> {
        let database = DatabaseConfig {
            host: var("DATABASE_HOST")?,
            port: var("DATABASE_PORT")?.parse()?,
            username: var("DATABASE_USERNAME")?,
            password: var("DATABASE_PASSWORD")?,
            database: var("DATABASE_NAME")?,
        };

        let redis = RedisConfig {
            host: var("REDIS_HOST")?,
            port: var("REDIS_PORT")?.parse::<u16>()?,
        };

        let auth = AuthConfig {
            ttl: var("AUTH_TOKEN_TTL")?.parse::<u64>()?,
        };
        Ok(Self {
            database,
            redis,
            auth,
        })
    }
}

#[derive(Debug)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
}

pub struct AuthConfig {
    pub ttl: u64,
}
