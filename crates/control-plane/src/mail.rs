//! Sending mail, without deciding yet who sends it.
//!
//! R1 needs exactly one message: the sign-in link. Which provider carries it
//! is still open (`docs/architecture.md` lists the email provider as chosen
//! during development), so everything talks to [`Mailer`] and the choice
//! stays a one-line change.
//!
//! Locally the link is written to the log. That is deliberate: signing in
//! during development needs no account with anybody.

use std::sync::{Arc, Mutex};

/// One message we know how to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignInLink {
    /// Who it goes to.
    pub email: String,
    /// The URL that signs them in. **Never log this.**
    pub url: String,
    /// Whether this address has an account yet, so the copy can differ.
    pub is_new: bool,
}

/// Anything that can deliver a message.
#[async_trait::async_trait]
pub trait Mailer: Send + Sync + std::fmt::Debug {
    /// Deliver a sign-in link.
    ///
    /// # Errors
    ///
    /// Returns an error if the message could not be handed over. Callers must
    /// not tell the browser which address failed, or the failure itself
    /// becomes a way to find out who has an account.
    async fn send_sign_in_link(&self, link: &SignInLink) -> Result<(), Error>;
}

/// Why a message could not be sent.
#[derive(Debug, thiserror::Error)]
#[error("could not send mail")]
pub struct Error(#[source] pub Box<dyn std::error::Error + Send + Sync>);

/// Writes the link to the log instead of sending it. The local default.
#[derive(Debug, Default)]
pub struct LogMailer;

#[async_trait::async_trait]
impl Mailer for LogMailer {
    async fn send_sign_in_link(&self, link: &SignInLink) -> Result<(), Error> {
        // The one place a live sign-in link is deliberately printed. It only
        // ever runs where a developer is the only reader; a deployment uses a
        // real mailer.
        tracing::info!(
            email = link.email,
            is_new = link.is_new,
            "sign-in link (development only): {}",
            link.url
        );

        Ok(())
    }
}

/// Keeps what was sent, so a test can read the link out.
#[derive(Debug, Default, Clone)]
pub struct TestMailer {
    sent: Arc<Mutex<Vec<SignInLink>>>,
}

impl TestMailer {
    /// Everything sent so far, oldest first.
    #[must_use]
    pub fn sent(&self) -> Vec<SignInLink> {
        self.locked().clone()
    }

    /// The most recent message, if any.
    #[must_use]
    pub fn last(&self) -> Option<SignInLink> {
        self.locked().last().cloned()
    }

    /// The messages, recovering the lock if a test thread panicked while
    /// holding it. A poisoned lock here says a test failed, and panicking a
    /// second time would only hide the first failure.
    fn locked(&self) -> std::sync::MutexGuard<'_, Vec<SignInLink>> {
        self.sent
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[async_trait::async_trait]
impl Mailer for TestMailer {
    async fn send_sign_in_link(&self, link: &SignInLink) -> Result<(), Error> {
        self.locked().push(link.clone());
        Ok(())
    }
}

/// The mailer a deployment uses when no provider is configured yet: it
/// refuses, loudly, rather than pretending to have sent something.
#[derive(Debug, Default)]
pub struct UnconfiguredMailer;

#[async_trait::async_trait]
impl Mailer for UnconfiguredMailer {
    async fn send_sign_in_link(&self, _link: &SignInLink) -> Result<(), Error> {
        Err(Error(
            "no email provider is configured, so no sign-in link can be sent".into(),
        ))
    }
}

/// The mailer to use for an environment.
#[must_use]
pub fn for_environment(environment: croncave_telemetry::Environment) -> Arc<dyn Mailer> {
    if environment.is_deployed() {
        // Staging and production must not quietly log links where anyone with
        // log access could use them.
        Arc::new(UnconfiguredMailer)
    } else {
        Arc::new(LogMailer)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use croncave_telemetry::Environment;

    fn link() -> SignInLink {
        SignInLink {
            email: "founder@example.com".to_owned(),
            url: "https://app.croncave.com/auth/callback?token=abc".to_owned(),
            is_new: false,
        }
    }

    #[tokio::test]
    async fn the_test_mailer_keeps_what_it_was_given() {
        let mailer = TestMailer::default();
        mailer.send_sign_in_link(&link()).await.unwrap();

        assert_eq!(mailer.sent().len(), 1);
        assert_eq!(mailer.last().unwrap().email, "founder@example.com");
    }

    #[tokio::test]
    async fn a_deployment_without_a_provider_refuses_rather_than_logging() {
        let mailer = for_environment(Environment::Production);

        let sent = mailer.send_sign_in_link(&link()).await;

        assert!(
            sent.is_err(),
            "a live sign-in link must never be written to a deployment's logs"
        );
    }

    #[tokio::test]
    async fn locally_the_link_goes_to_the_log() {
        let mailer = for_environment(Environment::Local);

        assert!(mailer.send_sign_in_link(&link()).await.is_ok());
    }
}
