//! What the control plane needs to know before it can start.

use std::net::SocketAddr;

use crate::Error;

/// Where the server listens.
pub const ENV_BIND: &str = "CRONCAVE_BIND";
/// The Postgres connection string.
pub const ENV_DATABASE_URL: &str = "DATABASE_URL";
/// How many Postgres connections to keep.
pub const ENV_DB_MAX_CONNECTIONS: &str = "CRONCAVE_DB_MAX_CONNECTIONS";
/// Where the browser reaches the app. Sign-in links point here.
pub const ENV_APP_URL: &str = "CRONCAVE_APP_URL";
/// Which environment this is, shared with `croncave-telemetry`.
pub const ENV_ENVIRONMENT: &str = "CRONCAVE_ENV";
/// Where workspaces' computers come from.
pub const ENV_COMPUTE_DRIVER: &str = "CRONCAVE_COMPUTE_DRIVER";

/// Loopback by default: the control plane is reached through the web app, and
/// nothing should bind a public interface by accident.
pub const DEFAULT_BIND: &str = "127.0.0.1:8080";
/// Enough for local work; deployments set their own.
pub const DEFAULT_DB_MAX_CONNECTIONS: u32 = 5;
/// The SvelteKit dev server, which is where a person's browser actually is.
pub const DEFAULT_APP_URL: &str = "http://localhost:5173";

/// Where a workspace's computer comes from.
///
/// One value per driver in `croncave-compute`. `fly` joins them once that
/// account and its terms are settled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ComputeDriverChoice {
    /// In memory. Fast, and real enough for everything above the driver.
    #[default]
    Fake,
    /// Docker on this machine.
    Local,
}

impl ComputeDriverChoice {
    /// The name used in configuration.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fake => "fake",
            Self::Local => "local",
        }
    }
}

impl std::str::FromStr for ComputeDriverChoice {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "fake" => Ok(Self::Fake),
            "local" | "docker" => Ok(Self::Local),
            other => Err(Error::UnknownComputeDriver(other.to_owned())),
        }
    }
}

/// Everything [`crate::serve`] needs.
///
/// Its [`std::fmt::Debug`] never prints the database URL, which carries a
/// password.
#[derive(Clone)]
pub struct Config {
    /// The address to listen on.
    pub bind: SocketAddr,
    /// How to reach Postgres.
    pub database_url: String,
    /// The size of the connection pool.
    pub db_max_connections: u32,
    /// Where the browser reaches the app, with no trailing slash.
    pub app_url: String,
    /// Which environment this is. Decides whether cookies are `Secure` and
    /// how sign-in links are delivered.
    pub environment: croncave_telemetry::Environment,
    /// Where workspaces' computers come from.
    pub compute_driver: ComputeDriverChoice,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("bind", &self.bind)
            .field("database_url", &"<set>")
            .field("db_max_connections", &self.db_max_connections)
            .field("app_url", &self.app_url)
            .field("environment", &self.environment)
            .field("compute_driver", &self.compute_driver)
            .finish()
    }
}

impl Config {
    /// Read the configuration from the environment.
    ///
    /// # Errors
    ///
    /// Returns an error if `DATABASE_URL` is missing, or if a value that is
    /// set cannot be understood. An unrecognised value stops the service
    /// rather than being guessed at, the same rule the rest of the codebase
    /// follows.
    pub fn from_env() -> Result<Self, Error> {
        Self::from_vars(|key| std::env::var(key).ok())
    }

    /// The body of [`Config::from_env`], with the environment injected so it
    /// can be tested without touching process state.
    fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Self, Error> {
        let database_url = var(ENV_DATABASE_URL)
            .filter(|value| !value.trim().is_empty())
            .ok_or(Error::MissingDatabaseUrl)?;

        let bind = var(ENV_BIND)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BIND.to_owned());
        let bind = bind
            .trim()
            .parse()
            .map_err(|_| Error::InvalidBind(bind.clone()))?;

        let db_max_connections = match var(ENV_DB_MAX_CONNECTIONS) {
            Some(value) if !value.trim().is_empty() => value
                .trim()
                .parse()
                .map_err(|_| Error::InvalidMaxConnections(value))?,
            _ => DEFAULT_DB_MAX_CONNECTIONS,
        };

        let app_url = var(ENV_APP_URL)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_APP_URL.to_owned());
        // A trailing slash would produce "…//auth/callback" in every link.
        let app_url = app_url.trim().trim_end_matches('/').to_owned();

        let environment = match var(ENV_ENVIRONMENT) {
            Some(value) if !value.trim().is_empty() => value
                .parse()
                .map_err(|_| Error::InvalidEnvironment(value))?,
            _ => croncave_telemetry::Environment::default(),
        };

        let compute_driver = match var(ENV_COMPUTE_DRIVER) {
            Some(value) if !value.trim().is_empty() => value.parse()?,
            _ => ComputeDriverChoice::default(),
        };

        Ok(Self {
            bind,
            database_url,
            db_max_connections,
            app_url,
            environment,
            compute_driver,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.to_owned())
        }
    }

    const URL: &str = "postgres://croncave:croncave@localhost:5432/croncave";

    #[test]
    fn defaults_apply_when_only_the_database_is_set() {
        let config = Config::from_vars(vars(&[(ENV_DATABASE_URL, URL)])).unwrap();

        assert_eq!(config.bind.to_string(), DEFAULT_BIND);
        assert_eq!(config.db_max_connections, DEFAULT_DB_MAX_CONNECTIONS);
    }

    #[test]
    fn the_default_bind_is_loopback() {
        let config = Config::from_vars(vars(&[(ENV_DATABASE_URL, URL)])).unwrap();

        assert!(
            config.bind.ip().is_loopback(),
            "nothing should bind a public interface by accident"
        );
    }

    #[test]
    fn a_missing_database_url_stops_the_service() {
        let error = Config::from_vars(vars(&[])).unwrap_err();

        assert!(matches!(error, Error::MissingDatabaseUrl), "{error}");
    }

    #[test]
    fn a_bind_address_that_makes_no_sense_is_refused() {
        let error = Config::from_vars(vars(&[
            (ENV_DATABASE_URL, URL),
            (ENV_BIND, "everywhere, please"),
        ]))
        .unwrap_err();

        assert!(matches!(error, Error::InvalidBind(_)), "{error}");
    }

    #[test]
    fn the_compute_driver_defaults_to_the_fake_one() {
        let config = Config::from_vars(vars(&[(ENV_DATABASE_URL, URL)])).unwrap();

        assert_eq!(config.compute_driver, ComputeDriverChoice::Fake);
    }

    #[test]
    fn the_compute_driver_can_be_chosen() {
        let config = Config::from_vars(vars(&[
            (ENV_DATABASE_URL, URL),
            (ENV_COMPUTE_DRIVER, "local"),
        ]))
        .unwrap();

        assert_eq!(config.compute_driver, ComputeDriverChoice::Local);
    }

    #[test]
    fn a_compute_driver_we_do_not_have_is_refused() {
        let error = Config::from_vars(vars(&[
            (ENV_DATABASE_URL, URL),
            (ENV_COMPUTE_DRIVER, "fly"),
        ]))
        .unwrap_err();

        // Better to stop than to quietly run everything on the fake.
        assert!(matches!(error, Error::UnknownComputeDriver(_)), "{error}");
    }

    #[test]
    fn a_pool_size_that_is_not_a_number_is_refused() {
        let error = Config::from_vars(vars(&[
            (ENV_DATABASE_URL, URL),
            (ENV_DB_MAX_CONNECTIONS, "lots"),
        ]))
        .unwrap_err();

        assert!(matches!(error, Error::InvalidMaxConnections(_)), "{error}");
    }

    #[test]
    fn debugging_a_config_never_prints_the_database_password() {
        let config = Config::from_vars(vars(&[(ENV_DATABASE_URL, URL)])).unwrap();

        let printed = format!("{config:?}");
        assert!(!printed.contains("croncave:croncave"), "{printed}");
        assert!(printed.contains("<set>"), "{printed}");
    }
}
