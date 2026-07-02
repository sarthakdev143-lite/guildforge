//! Mock-HTTP integration tests for the Discord provider.
//!
//! These tests spin up a `wiremock` server that mocks the Discord REST
//! API and verify that our CRUD operations issue the correct requests
//! and decode the responses correctly.
//!
//! Live tests against real Discord live in `tests/live/` behind the
//! `live-discord` feature flag.

use guildforge_provider::Provider;
use guildforge_provider_discord::{
    client::{DiscordHttp, DiscordHttpConfig},
    DiscordProvider,
};
use guildforge_shared::{ResourceId, Snowflake};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn make_provider() -> (MockServer, DiscordProvider) {
    let server = MockServer::start().await;
    let config = DiscordHttpConfig {
        api_base: server.uri() + "/api/v10",
        timeout: std::time::Duration::from_secs(5),
        max_retries: 0, // make tests fast
        ..Default::default()
    };
    let http = DiscordHttp::new("test-token".into(), config).unwrap();
    let provider = DiscordProvider::new(Snowflake::new(123), std::sync::Arc::new(http));
    (server, provider)
}

fn discord_role(id: &str, name: &str, color: u32, perms: &str) -> serde_json::Value {
    json!({
        "id": id,
        "name": name,
        "color": color,
        "hoist": false,
        "mentionable": false,
        "permissions": perms,
        "position": 1,
    })
}

// ===========================================================================
// Role tests
// ===========================================================================

#[tokio::test]
async fn role_list_decodes_response() {
    let (server, provider) = make_provider().await;
    Mock::given(method("GET"))
        .and(path("/api/v10/guilds/123/roles"))
        .and(header("Authorization", "Bot test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            discord_role("1", "@everyone", 0, "0"),
            discord_role("2", "Admin", 16711680, "8"),
        ])))
        .mount(&server)
        .await;

    let roles = guildforge_provider_discord::resources::role::list(&provider)
        .await
        .unwrap();
    assert_eq!(roles.len(), 2);
    assert_eq!(roles[1].name, "Admin");
    assert_eq!(roles[1].permissions, 8);
    assert_eq!(roles[1].id, Some(Snowflake::new(2)));
}

#[tokio::test]
async fn role_read_finds_by_name_case_insensitive() {
    let (server, provider) = make_provider().await;
    Mock::given(method("GET"))
        .and(path("/api/v10/guilds/123/roles"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            discord_role("1", "@everyone", 0, "0"),
            discord_role("2", "Admin", 0, "8"),
        ])))
        .mount(&server)
        .await;

    let addr = ResourceId::new("role/admin");
    let r = guildforge_provider_discord::resources::role::read(&provider, &addr)
        .await
        .unwrap();
    assert!(r.is_some());
    assert_eq!(r.unwrap().name, "Admin");
}

#[tokio::test]
async fn role_read_returns_none_if_not_found() {
    let (server, provider) = make_provider().await;
    Mock::given(method("GET"))
        .and(path("/api/v10/guilds/123/roles"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!([discord_role(
                "1",
                "@everyone",
                0,
                "0"
            ),])),
        )
        .mount(&server)
        .await;

    let addr = ResourceId::new("role/Ghost");
    let r = guildforge_provider_discord::resources::role::read(&provider, &addr)
        .await
        .unwrap();
    assert!(r.is_none());
}

#[tokio::test]
async fn role_create_posts_correct_payload() {
    let (server, provider) = make_provider().await;
    Mock::given(method("POST"))
        .and(path("/api/v10/guilds/123/roles"))
        .and(wiremock::matchers::body_partial_json(json!({
            "name": "Admin",
            "color": 16711680,
            "hoist": true,
            "mentionable": true,
            "permissions": "8",
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(discord_role("10", "Admin", 16711680, "8")),
        )
        .mount(&server)
        .await;

    let desired = guildforge_provider::RoleResource {
        addr: ResourceId::new("role/Admin"),
        id: None,
        name: "Admin".into(),
        color: 0xFF0000,
        hoist: true,
        mentionable: true,
        permissions: 8,
        position: 0,
        unicode_emoji: None,
    };
    let r = guildforge_provider_discord::resources::role::create(&provider, &desired)
        .await
        .unwrap();
    assert_eq!(r.id, Some(Snowflake::new(10)));
    assert_eq!(r.name, "Admin");
}

#[tokio::test]
async fn role_delete_returns_ok_on_404() {
    let (server, provider) = make_provider().await;
    Mock::given(method("DELETE"))
        .and(path("/api/v10/guilds/123/roles/999"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;

    let current = guildforge_provider::RoleResource {
        addr: ResourceId::new("role/Ghost"),
        id: Some(Snowflake::new(999)),
        name: "Ghost".into(),
        color: 0,
        hoist: false,
        mentionable: false,
        permissions: 0,
        position: 0,
        unicode_emoji: None,
    };
    let r = guildforge_provider_discord::resources::role::delete(&provider, &current).await;
    assert!(r.is_ok(), "404 should be Ok (idempotent delete): {r:?}");
}

// ===========================================================================
// Channel tests
// ===========================================================================

#[tokio::test]
async fn channel_list_decodes_response() {
    let (server, provider) = make_provider().await;
    Mock::given(method("GET"))
        .and(path("/api/v10/guilds/123/channels"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "1", "type": 0, "name": "general", "position": 0,
                "permission_overwrites": [], "nsfw": false, "rate_limit_per_user": 0,
            },
            {
                "id": "2", "type": 4, "name": "CATEGORY", "position": 0,
                "permission_overwrites": [],
            },
            {
                "id": "3", "type": 2, "name": "Voice", "position": 1,
                "permission_overwrites": [], "bitrate": 64000, "user_limit": 0,
            },
        ])))
        .mount(&server)
        .await;

    let channels = guildforge_provider_discord::resources::channel::list(&provider)
        .await
        .unwrap();
    // Categories (type=4) are filtered out by `list`.
    assert_eq!(channels.len(), 2);
    assert_eq!(channels[0].name, "general");
    assert_eq!(channels[0].kind, guildforge_provider::ChannelType::Text);
    assert_eq!(channels[1].name, "Voice");
}

#[tokio::test]
async fn channel_create_posts_correct_payload() {
    let (server, provider) = make_provider().await;
    Mock::given(method("POST"))
        .and(path("/api/v10/guilds/123/channels"))
        .and(wiremock::matchers::body_partial_json(json!({
            "name": "general",
            "type": 0,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "100", "type": 0, "name": "general", "position": 0,
            "permission_overwrites": [], "nsfw": false, "rate_limit_per_user": 0,
        })))
        .mount(&server)
        .await;

    let desired = guildforge_provider::ChannelResource::new_text("channel/general", "general");
    let r = guildforge_provider_discord::resources::channel::create(&provider, &desired)
        .await
        .unwrap();
    assert_eq!(r.id, Some(Snowflake::new(100)));
}

#[tokio::test]
async fn category_create_uses_type_4() {
    let (server, provider) = make_provider().await;
    Mock::given(method("POST"))
        .and(path("/api/v10/guilds/123/channels"))
        .and(wiremock::matchers::body_partial_json(json!({
            "name": "COMPANY",
            "type": 4,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "200", "type": 4, "name": "COMPANY", "position": 0,
            "permission_overwrites": [],
        })))
        .mount(&server)
        .await;

    let desired = guildforge_provider::CategoryResource::new("category/COMPANY", "COMPANY");
    let r = guildforge_provider_discord::resources::channel::create_category(&provider, &desired)
        .await
        .unwrap();
    assert_eq!(r.id, Some(Snowflake::new(200)));
}

#[tokio::test]
async fn channel_delete_returns_ok_on_404() {
    let (server, provider) = make_provider().await;
    Mock::given(method("DELETE"))
        .and(path("/api/v10/channels/999"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;

    let r = guildforge_provider_discord::resources::channel::delete_channel(
        &provider,
        Some(Snowflake::new(999)),
    )
    .await;
    assert!(r.is_ok(), "404 should be Ok: {r:?}");
}

// ===========================================================================
// Provider trait tests
// ===========================================================================

#[tokio::test]
async fn provider_name_is_discord() {
    let (_server, provider) = make_provider().await;
    assert_eq!(provider.name(), "discord");
}

#[tokio::test]
async fn provider_list_roles_dispatches_correctly() {
    let (server, provider) = make_provider().await;
    Mock::given(method("GET"))
        .and(path("/api/v10/guilds/123/roles"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!([discord_role(
                "1",
                "@everyone",
                0,
                "0"
            ),])),
        )
        .mount(&server)
        .await;

    let resources = provider
        .list(guildforge_provider::ResourceKind::Role)
        .await
        .unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].kind(), guildforge_provider::ResourceKind::Role);
}

#[tokio::test]
async fn provider_read_returns_none_for_unknown_addr() {
    let (server, provider) = make_provider().await;
    Mock::given(method("GET"))
        .and(path("/api/v10/guilds/123/roles"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;

    let addr = ResourceId::new("role/Ghost");
    let r = provider.read(&addr).await.unwrap();
    assert!(r.is_none());
}

// ===========================================================================
// Rate limit / retry behavior tests
// ===========================================================================

#[tokio::test]
async fn http_retries_on_5xx() {
    let server = MockServer::start().await;
    let config = DiscordHttpConfig {
        api_base: server.uri() + "/api/v10",
        timeout: std::time::Duration::from_secs(5),
        max_retries: 2,
        initial_backoff: std::time::Duration::from_millis(10),
        max_backoff: std::time::Duration::from_millis(100),
        ..Default::default()
    };
    let http = DiscordHttp::new("test-token".into(), config).unwrap();
    let provider = DiscordProvider::new(Snowflake::new(123), std::sync::Arc::new(http));

    // First two calls return 500; third returns 200.
    Mock::given(method("GET"))
        .and(path("/api/v10/guilds/123/roles"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v10/guilds/123/roles"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;

    let r = guildforge_provider_discord::resources::role::list(&provider).await;
    assert!(r.is_ok(), "expected retry to succeed: {r:?}");
}

#[tokio::test]
async fn http_returns_permanent_error_on_4xx_other_than_429() {
    let server = MockServer::start().await;
    let config = DiscordHttpConfig {
        api_base: server.uri() + "/api/v10",
        timeout: std::time::Duration::from_secs(5),
        max_retries: 0,
        ..Default::default()
    };
    let http = DiscordHttp::new("test-token".into(), config).unwrap();
    let provider = DiscordProvider::new(Snowflake::new(123), std::sync::Arc::new(http));

    Mock::given(method("GET"))
        .and(path("/api/v10/guilds/123/roles"))
        .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
        .mount(&server)
        .await;

    let r = guildforge_provider_discord::resources::role::list(&provider).await;
    assert!(r.is_err());
    let e = r.unwrap_err();
    assert!(
        matches!(
            e,
            guildforge_provider_discord::DiscordError::Discord { status: 403, .. }
        ),
        "expected 403, got {e:?}"
    );
}
