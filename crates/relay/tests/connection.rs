//! A real agent talking to a real relay, both in this process.
//!
//! The transport is a real WebSocket over a real socket on loopback, so
//! these exercise the multiplexer, the framing and the hello exchange rather
//! than a stand-in for them.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::{State, WebSocketUpgrade};
use axum::routing::any;
use croncave_agent::Config;
use croncave_protocol::{Credential, Outcome, Refusal};
use croncave_relay::{Authorised, Authoriser, Error, Relay};
use uuid::Uuid;

/// An authoriser that accepts one token, for one workspace.
#[derive(Debug)]
struct OneWorkspace {
    workspace_id: Uuid,
    token: String,
}

#[async_trait::async_trait]
impl Authoriser for OneWorkspace {
    async fn authorise(&self, credential: &Credential) -> Result<Authorised, Refusal> {
        let (Credential::Bootstrap { token } | Credential::Workspace { token }) = credential;

        if token == &self.token {
            Ok(Authorised {
                workspace_id: self.workspace_id,
                fresh_credential: Some("a-longer-lived-credential".to_owned()),
            })
        } else {
            Err(Refusal::NotAuthorised)
        }
    }
}

/// Start a relay on a port of its own and return it with its address.
async fn start_relay(authoriser: Arc<dyn Authoriser>) -> (Relay, String) {
    let relay = Relay::new(authoriser);

    let app = Router::new()
        .route(
            "/agent",
            any(
                |State(relay): State<Relay>, upgrade: WebSocketUpgrade| async move {
                    upgrade.on_upgrade(move |socket| croncave_relay::serve_agent(relay, socket))
                },
            ),
        )
        .with_state(relay.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (relay, format!("ws://{address}/agent"))
}

/// Wait for a workspace to show up, or give up.
async fn wait_connected(relay: &Relay, workspace_id: Uuid) -> bool {
    for _ in 0..100 {
        if relay.is_connected(workspace_id).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

/// A relay and a connected agent, ready to be asked things.
async fn connected() -> (Relay, Uuid) {
    let workspace_id = Uuid::now_v7();
    let authoriser = Arc::new(OneWorkspace {
        workspace_id,
        token: "the-right-token".to_owned(),
    });

    let (relay, url) = start_relay(authoriser).await;

    tokio::spawn(croncave_agent::run_forever(Config {
        relay_url: url,
        bootstrap_token: "the-right-token".to_owned(),
        agent_version: "test".to_owned(),
    }));

    assert!(
        wait_connected(&relay, workspace_id).await,
        "the agent never connected"
    );

    (relay, workspace_id)
}

#[tokio::test]
async fn an_agent_connects_and_answers() {
    let (relay, workspace_id) = connected().await;

    relay
        .ping(workspace_id)
        .await
        .expect("a workspace should answer a ping");
}

#[tokio::test]
async fn a_workspace_runs_a_command_and_sends_its_output_back() {
    let (relay, workspace_id) = connected().await;

    let ran = relay
        .run(
            workspace_id,
            "echo",
            &["hello from inside".to_owned()],
            Duration::from_secs(10),
        )
        .await
        .expect("the command should run");

    assert!(ran.output.contains("hello from inside"), "{:?}", ran.output);
    assert!(ran.outcome.succeeded(), "{:?}", ran.outcome);
}

#[tokio::test]
async fn a_command_that_fails_reports_its_exit_code() {
    let (relay, workspace_id) = connected().await;

    let ran = relay
        .run(
            workspace_id,
            "sh",
            &["-c".to_owned(), "echo nope >&2; exit 3".to_owned()],
            Duration::from_secs(10),
        )
        .await
        .expect("it should run, even though it fails");

    assert!(ran.output.contains("nope"), "stderr comes back too");
    assert_eq!(
        ran.outcome,
        Outcome::Exited {
            code: 3,
            truncated: false
        }
    );
    assert!(!ran.outcome.succeeded());
}

#[tokio::test]
async fn a_command_that_runs_too_long_is_stopped() {
    let (relay, workspace_id) = connected().await;

    let ran = relay
        .run(
            workspace_id,
            "sleep",
            &["30".to_owned()],
            // A workspace runs code we did not write; nothing may run for
            // ever by accident.
            Duration::from_secs(1),
        )
        .await
        .expect("it should come back");

    assert_eq!(ran.outcome, Outcome::TimedOut);
}

#[tokio::test]
async fn a_command_that_does_not_exist_says_so_rather_than_hanging() {
    let (relay, workspace_id) = connected().await;

    let ran = relay
        .run(
            workspace_id,
            "definitely-not-a-program",
            &[],
            Duration::from_secs(10),
        )
        .await
        .expect("it should answer");

    assert!(!ran.outcome.succeeded());
    assert!(
        ran.output.contains("could not run"),
        "the reason should come back: {:?}",
        ran.output
    );
}

#[tokio::test]
async fn a_workspace_that_is_not_connected_says_so() {
    let (relay, _) = connected().await;

    let result = relay
        .run(Uuid::now_v7(), "echo", &[], Duration::from_secs(5))
        .await;

    // Not a hang and not a generic failure: "not connected" is the answer a
    // caller can act on, by waking the workspace.
    assert!(matches!(result, Err(Error::NotConnected)), "{result:?}");
}

#[tokio::test]
async fn an_agent_with_the_wrong_token_is_refused() {
    let workspace_id = Uuid::now_v7();
    let (relay, url) = start_relay(Arc::new(OneWorkspace {
        workspace_id,
        token: "the-right-token".to_owned(),
    }))
    .await;

    tokio::spawn(croncave_agent::run_forever(Config {
        relay_url: url,
        bootstrap_token: "not-the-right-token".to_owned(),
        agent_version: "test".to_owned(),
    }));

    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(
        !relay.is_connected(workspace_id).await,
        "a workspace must not be reachable on a credential the relay refused"
    );
    assert_eq!(relay.connected_count().await, 0);
}

#[tokio::test]
async fn an_agent_reconnects_after_the_relay_goes_away() {
    let workspace_id = Uuid::now_v7();
    let authoriser = Arc::new(OneWorkspace {
        workspace_id,
        token: "the-right-token".to_owned(),
    });

    // Bind a fixed port so the relay can be taken away and put back.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);

    tokio::spawn(croncave_agent::run_forever(Config {
        relay_url: format!("ws://{address}/agent"),
        bootstrap_token: "the-right-token".to_owned(),
        agent_version: "test".to_owned(),
    }));

    // The agent starts talking to nothing at all, and must keep trying.
    tokio::time::sleep(Duration::from_millis(400)).await;

    let relay = Relay::new(authoriser);
    let app = Router::new()
        .route(
            "/agent",
            any(
                |State(relay): State<Relay>, upgrade: WebSocketUpgrade| async move {
                    upgrade.on_upgrade(move |socket| croncave_relay::serve_agent(relay, socket))
                },
            ),
        )
        .with_state(relay.clone());

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    assert!(
        wait_connected(&relay, workspace_id).await,
        "the agent should find the relay once it exists"
    );
}
