//! Test-only doubles for `X402Client`'s orchestration tests: a mock x402
//! resource server (402 with `PAYMENT-REQUIRED`, then 200 on the signed
//! retry) and a deterministic signer.
//!
//! The client is a `reqwest::Client` with no trait seam, so the resource
//! server is stood in for at the HTTP boundary rather than behind a trait —
//! the real transport and header shapes are exercised without changing any
//! public signature.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use ed25519_dalek::{Signer as DalekSigner, SigningKey};
use sorocharge_signer::{Address, Signer, SorochargeError};
use stellar_xdr::{AccountId, PublicKey, ScAddress, Uint256};

use crate::client::{PAYMENT_REQUIRED_HEADER, PAYMENT_SIGNATURE_HEADER};
use crate::types::PaymentRequired;

/// A deterministic ed25519 signer whose reported address is derived from its
/// own key.
pub(crate) struct TestSigner {
    signing_key: SigningKey,
    address: Address,
}

impl TestSigner {
    pub(crate) fn from_seed(seed: [u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(&seed);
        let public_key = signing_key.verifying_key().to_bytes();
        let address = ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(
            public_key,
        ))));
        Self {
            signing_key,
            address,
        }
    }
}

impl Signer for TestSigner {
    fn sign_preimage(&self, preimage: &[u8]) -> Result<[u8; 64], SorochargeError> {
        Ok(self.signing_key.sign(preimage).to_bytes())
    }

    fn address(&self) -> Address {
        self.address.clone()
    }
}

/// A mock x402 resource server: `402` with a `PAYMENT-REQUIRED` header when
/// no `PAYMENT-SIGNATURE` is present, `200` (capturing the header) when it
/// is.
pub(crate) struct MockResource {
    pub(crate) url: String,
    pub(crate) state: Arc<MockResourceState>,
}

pub(crate) struct MockResourceState {
    payment_required: PaymentRequired,
    captured: Mutex<Option<String>>,
    requests: Mutex<u32>,
}

impl MockResource {
    pub(crate) async fn spawn(payment_required: PaymentRequired) -> Self {
        let state = Arc::new(MockResourceState {
            payment_required,
            captured: Mutex::new(None),
            requests: Mutex::new(0),
        });
        let router = Router::new()
            .route("/resource", get(handle_resource))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind an ephemeral localhost port");
        let addr = listener.local_addr().expect("read the bound address");
        tokio::spawn(async move {
            axum::serve(listener, router).await.expect("mock server");
        });
        Self {
            url: format!("http://{addr}/resource"),
            state,
        }
    }

    /// The `PAYMENT-SIGNATURE` header value the client sent on its retry.
    pub(crate) fn captured_signature(&self) -> String {
        self.state
            .captured
            .lock()
            .expect("captured lock")
            .clone()
            .expect("the client should have sent a PAYMENT-SIGNATURE header")
    }

    /// How many requests the resource received (1 = only the challenge).
    pub(crate) fn request_count(&self) -> u32 {
        *self.state.requests.lock().expect("requests lock")
    }
}

async fn handle_resource(
    State(state): State<Arc<MockResourceState>>,
    headers: HeaderMap,
) -> Response {
    *state.requests.lock().expect("requests lock") += 1;

    if let Some(signature) = headers.get(PAYMENT_SIGNATURE_HEADER) {
        *state.captured.lock().expect("captured lock") =
            Some(signature.to_str().expect("header is ASCII").to_string());
        return (StatusCode::OK, "paid resource body\n").into_response();
    }

    let encoded = BASE64
        .encode(serde_json::to_vec(&state.payment_required).expect("serialize PaymentRequired"));
    let mut out = HeaderMap::new();
    out.insert(
        PAYMENT_REQUIRED_HEADER,
        HeaderValue::from_str(&encoded).expect("base64 is a valid header value"),
    );
    (StatusCode::PAYMENT_REQUIRED, out).into_response()
}
