//! How a service decides what to log and where it is running.

use std::io::IsTerminal;
use std::str::FromStr;

use crate::Error;

/// Environment variable holding the `tracing` filter, e.g. `info,croncave_relay=debug`.
pub const ENV_FILTER: &str = "RUST_LOG";
/// Environment variable naming the environment the service runs in.
pub const ENV_ENVIRONMENT: &str = "CRONCAVE_ENV";
/// Environment variable choosing the log format.
pub const ENV_LOG_FORMAT: &str = "CRONCAVE_LOG_FORMAT";

/// The default filter when `RUST_LOG` is unset.
pub const DEFAULT_FILTER: &str = "info";

/// Where a service is running. Recorded on every JSON event so logs from
/// staging and production are never mistaken for one another.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Environment {
    /// A developer machine.
    #[default]
    Local,
    /// A CI run.
    Ci,
    /// The staging deployment.
    Staging,
    /// The production deployment.
    Production,
}

impl Environment {
    /// The lowercase name used in logs and in `CRONCAVE_ENV`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Ci => "ci",
            Self::Staging => "staging",
            Self::Production => "production",
        }
    }

    /// Whether this environment serves real users. Used to decide defaults
    /// that should be careful in production and convenient elsewhere.
    #[must_use]
    pub fn is_deployed(self) -> bool {
        matches!(self, Self::Staging | Self::Production)
    }
}

impl FromStr for Environment {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" | "dev" | "development" => Ok(Self::Local),
            "ci" | "test" => Ok(Self::Ci),
            "staging" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            other => Err(Error::UnknownEnvironment(other.to_owned())),
        }
    }
}

/// How log lines are written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable lines for a terminal.
    #[default]
    Pretty,
    /// One JSON object per line, for a log collector.
    Json,
}

impl LogFormat {
    /// The format to use when `CRONCAVE_LOG_FORMAT` is unset: pretty when a
    /// person is watching a terminal, JSON when something is collecting it.
    #[must_use]
    pub fn default_for_output(is_terminal: bool) -> Self {
        if is_terminal {
            Self::Pretty
        } else {
            Self::Json
        }
    }

    /// The lowercase name used in `CRONCAVE_LOG_FORMAT`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pretty => "pretty",
            Self::Json => "json",
        }
    }
}

impl FromStr for LogFormat {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pretty" | "text" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            other => Err(Error::UnknownLogFormat(other.to_owned())),
        }
    }
}

/// Everything [`crate::init`] needs to set a service's telemetry up.
#[derive(Clone, Debug)]
pub struct Config {
    /// The service's name, e.g. `control-plane`. Recorded on every JSON event.
    pub service: String,
    /// The build's version, normally `env!("CARGO_PKG_VERSION")`.
    pub version: String,
    /// Where this service is running.
    pub environment: Environment,
    /// How log lines are written.
    pub log_format: LogFormat,
    /// The `tracing` filter, in `RUST_LOG` syntax.
    pub filter: String,
}

impl Config {
    /// A configuration with the defaults: local, pretty, `info`.
    ///
    /// Pass `env!("CARGO_PKG_VERSION")` as the version from the binary's own
    /// crate, so the version reported is the one that was built.
    pub fn new(service: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            version: version.into(),
            environment: Environment::default(),
            log_format: LogFormat::default(),
            filter: DEFAULT_FILTER.to_owned(),
        }
    }

    /// Read the configuration from the environment, falling back to defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if `CRONCAVE_ENV` or `CRONCAVE_LOG_FORMAT` holds a
    /// value we don't recognise. A typo in a deployment's configuration should
    /// stop the service, not silently make it look like a local run.
    pub fn from_env(service: impl Into<String>, version: impl Into<String>) -> Result<Self, Error> {
        Self::from_vars(
            service,
            version,
            |key| std::env::var(key).ok(),
            || std::io::stdout().is_terminal(),
        )
    }

    /// The body of [`Config::from_env`], with the environment and the terminal
    /// check injected so it can be tested without touching process state.
    fn from_vars(
        service: impl Into<String>,
        version: impl Into<String>,
        var: impl Fn(&str) -> Option<String>,
        is_terminal: impl Fn() -> bool,
    ) -> Result<Self, Error> {
        let environment = match var(ENV_ENVIRONMENT) {
            Some(value) => value.parse()?,
            None => Environment::default(),
        };
        let log_format = match var(ENV_LOG_FORMAT) {
            Some(value) => value.parse()?,
            None => LogFormat::default_for_output(is_terminal()),
        };
        let filter = var(ENV_FILTER)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_FILTER.to_owned());

        Ok(Self {
            service: service.into(),
            version: version.into(),
            environment,
            log_format,
            filter,
        })
    }

    /// Override the environment. Useful in tests and in binaries that already
    /// know where they run.
    #[must_use]
    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }

    /// Override the log format.
    #[must_use]
    pub fn with_log_format(mut self, log_format: LogFormat) -> Self {
        self.log_format = log_format;
        self
    }

    /// Override the filter.
    #[must_use]
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = filter.into();
        self
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

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

    #[test]
    fn environment_parses_its_names_and_aliases() {
        assert_eq!("local".parse::<Environment>().unwrap(), Environment::Local);
        assert_eq!("CI".parse::<Environment>().unwrap(), Environment::Ci);
        assert_eq!(
            " staging ".parse::<Environment>().unwrap(),
            Environment::Staging
        );
        assert_eq!(
            "prod".parse::<Environment>().unwrap(),
            Environment::Production
        );
    }

    #[test]
    fn unknown_environment_is_rejected() {
        let error = "eu-west".parse::<Environment>().unwrap_err();
        assert!(matches!(error, Error::UnknownEnvironment(value) if value == "eu-west"));
    }

    #[test]
    fn unknown_log_format_is_rejected() {
        let error = "yaml".parse::<LogFormat>().unwrap_err();
        assert!(matches!(error, Error::UnknownLogFormat(value) if value == "yaml"));
    }

    #[test]
    fn log_format_follows_the_output_when_unset() {
        assert_eq!(LogFormat::default_for_output(true), LogFormat::Pretty);
        assert_eq!(LogFormat::default_for_output(false), LogFormat::Json);
    }

    #[test]
    fn defaults_apply_when_nothing_is_set() {
        let config = Config::from_vars("relay", "0.1.0", vars(&[]), || false).unwrap();

        assert_eq!(config.service, "relay");
        assert_eq!(config.version, "0.1.0");
        assert_eq!(config.environment, Environment::Local);
        assert_eq!(config.log_format, LogFormat::Json);
        assert_eq!(config.filter, DEFAULT_FILTER);
    }

    #[test]
    fn environment_variables_win_over_defaults() {
        let config = Config::from_vars(
            "relay",
            "0.1.0",
            vars(&[
                (ENV_ENVIRONMENT, "production"),
                (ENV_LOG_FORMAT, "pretty"),
                (ENV_FILTER, "warn,croncave_relay=debug"),
            ]),
            || false,
        )
        .unwrap();

        assert_eq!(config.environment, Environment::Production);
        assert!(config.environment.is_deployed());
        assert_eq!(config.log_format, LogFormat::Pretty);
        assert_eq!(config.filter, "warn,croncave_relay=debug");
    }

    #[test]
    fn a_blank_filter_falls_back_to_the_default() {
        let config =
            Config::from_vars("relay", "0.1.0", vars(&[(ENV_FILTER, "  ")]), || false).unwrap();

        assert_eq!(config.filter, DEFAULT_FILTER);
    }
}
