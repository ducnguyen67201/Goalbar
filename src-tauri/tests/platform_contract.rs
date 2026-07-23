#![allow(clippy::unwrap_used)]

use tagline_lib::adapters::platform::http::HttpTransport;
use tagline_lib::adapters::platform::linkedin::LinkedInAdapter;
use tagline_lib::adapters::platform::reddit::RedditAdapter;
use tagline_lib::adapters::platform::x::XAdapter;
use tagline_lib::adapters::platform::{
    DirectMessageRequest, PlatformAdapter, PlatformRequestContext, PublishRequest,
};
use tagline_lib::domain::{CapabilityState, Platform};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn context() -> PlatformRequestContext {
    PlatformRequestContext {
        access_token: "test-token".to_owned(),
        account_id: "123".to_owned(),
        display_name: "founder".to_owned(),
        scopes: vec!["tweet.write".to_owned(), "submit".to_owned()],
    }
}

#[tokio::test]
async fn x_contract_publishes_normalized_content() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/2/tweets"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {"id": "post-1", "text": "A founder insight"}
        })))
        .mount(&server)
        .await;
    let adapter = XAdapter::new(HttpTransport::production(), server.uri());
    let result = adapter
        .publish(
            &context(),
            PublishRequest {
                body: "A founder insight".to_owned(),
                title: None,
                destination: None,
                reply_to_id: None,
                idempotency_key: "idempotent".to_owned(),
            },
        )
        .await
        .expect("publish");
    assert_eq!(result.platform, Platform::X);
    assert_eq!(result.remote_id, "post-1");
}

#[tokio::test]
async fn reddit_contract_requires_destination() {
    let adapter = RedditAdapter::new(HttpTransport::production(), "https://example.invalid");
    let result = adapter
        .publish(
            &context(),
            PublishRequest {
                body: "Body".to_owned(),
                title: Some("Title".to_owned()),
                destination: None,
                reply_to_id: None,
                idempotency_key: "idempotent".to_owned(),
            },
        )
        .await;
    assert!(matches!(
        result,
        Err(tagline_lib::error::AppError::Validation(_))
    ));
}

#[tokio::test]
async fn linkedin_contract_is_honest_about_direct_messages() {
    let adapter = LinkedInAdapter::new(
        HttpTransport::production(),
        "https://example.invalid",
        "202606",
    );
    assert_eq!(
        adapter.capabilities(&[]).direct_messages,
        CapabilityState::Unsupported
    );
    let result = adapter
        .send_direct_message(
            &context(),
            DirectMessageRequest {
                recipient_id: "person".to_owned(),
                body: "Hello".to_owned(),
                idempotency_key: "idempotent".to_owned(),
            },
        )
        .await;
    assert!(matches!(
        result,
        Err(tagline_lib::error::AppError::Unsupported(_))
    ));
}
