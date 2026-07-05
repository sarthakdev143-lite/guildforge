//! Performance benchmarks for the planner.
//!
//! Run with: `cargo bench -p guildforge-planner`
//!
//! Verifies the ARCHITECTURE.md performance targets:
//! - `plan` on 500-channel guild < 1s

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use guildforge_config::{Channel, ChannelType, Config, Role, Server};
use guildforge_planner::{config_to_resources, Planner};
use guildforge_state::CurrentState;

/// Generate a config with N roles, M categories, each with K channels.
fn generate_config(n_roles: usize, m_categories: usize, k_channels: usize) -> Config {
    let roles: Vec<Role> = (0..n_roles)
        .map(|i| Role {
            name: format!("Role{i}"),
            color: None,
            hoist: None,
            mentionable: None,
            permissions: vec!["send_messages".to_string()],
            position: None,
            icon: None,
            unicode_emoji: None,
        })
        .collect();

    let categories: Vec<guildforge_config::Category> = (0..m_categories)
        .map(|c| guildforge_config::Category {
            name: format!("CAT{c}"),
            description: None,
            permissions: None,
            channels: (0..k_channels)
                .map(|k| Channel {
                    name: format!("ch{k}"),
                    kind: ChannelType::Text,
                    category: None,
                    topic: Some(format!("Channel {k} in CAT{c}")),
                    nsfw: None,
                    slowmode: None,
                    permissions: None,
                    text: None,
                    voice: None,
                    stage: None,
                    forum: None,
                    announcement: None,
                })
                .collect(),
        })
        .collect();

    Config {
        schema_version: Some(1),
        server: Server {
            name: "BenchmarkGuild".to_string(),
            description: None,
            icon: None,
            banner: None,
            verification_level: None,
            explicit_content_filter: None,
            default_notifications: None,
            system_channel: None,
            system_channel_flags: vec![],
            afk_channel: None,
            afk_timeout: None,
            premium_progress_bar: None,
        },
        roles,
        categories,
        channels: vec![],
        permissions: std::collections::BTreeMap::new(),
        permission_overwrites: vec![],
        webhooks: vec![],
        invites: vec![],
        forum_tags: std::collections::BTreeMap::new(),
        welcome_screen: None,
        server_guide: None,
        ordering: None,
    }
}

fn bench_config_to_resources(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_to_resources");

    group.bench_function("10_roles_5_cats_5_chans", |b| {
        let cfg = generate_config(10, 5, 5);
        b.iter(|| black_box(config_to_resources(black_box(&cfg)).unwrap()))
    });

    group.bench_function("50_roles_20_cats_10_chans", |b| {
        let cfg = generate_config(50, 20, 10);
        b.iter(|| black_box(config_to_resources(black_box(&cfg)).unwrap()))
    });

    group.bench_function("100_roles_50_cats_10_chans", |b| {
        let cfg = generate_config(100, 50, 10);
        b.iter(|| black_box(config_to_resources(black_box(&cfg)).unwrap()))
    });

    group.finish();
}

fn bench_plan(c: &mut Criterion) {
    let mut group = c.benchmark_group("plan");
    group.sample_size(20);

    // Small guild: 10 roles, 5 categories, 5 channels each (35 resources)
    group.bench_function("small_guild_35_resources", |b| {
        let cfg = generate_config(10, 5, 5);
        let _desired = config_to_resources(&cfg).unwrap();
        let state = CurrentState::default();
        b.iter(|| {
            let planner = Planner::new();
            black_box(planner.plan(black_box(&cfg), black_box(&state)).unwrap())
        })
    });

    // Medium guild: 50 roles, 20 categories, 10 channels each (250 resources)
    group.bench_function("medium_guild_250_resources", |b| {
        let cfg = generate_config(50, 20, 10);
        let _desired = config_to_resources(&cfg).unwrap();
        let state = CurrentState::default();
        b.iter(|| {
            let planner = Planner::new();
            black_box(planner.plan(black_box(&cfg), black_box(&state)).unwrap())
        })
    });

    // Large guild: 100 roles, 50 categories, 10 channels each (600 resources)
    // This exceeds Discord's 500-channel limit but tests scaling.
    group.bench_function("large_guild_600_resources", |b| {
        let cfg = generate_config(100, 50, 10);
        let _desired = config_to_resources(&cfg).unwrap();
        let state = CurrentState::default();
        b.iter(|| {
            let planner = Planner::new();
            black_box(planner.plan(black_box(&cfg), black_box(&state)).unwrap())
        })
    });

    // The ARCHITECTURE.md target: plan on 500-channel guild < 1s.
    // 50 roles, 50 categories, 10 channels each = 550 resources.
    group.bench_function("target_500_channel_guild", |b| {
        let cfg = generate_config(50, 50, 10);
        let state = CurrentState::default();
        b.iter(|| {
            let planner = Planner::new();
            black_box(planner.plan(black_box(&cfg), black_box(&state)).unwrap())
        })
    });

    group.finish();
}

fn bench_plan_with_existing_state(c: &mut Criterion) {
    let mut group = c.benchmark_group("plan_with_state");
    group.sample_size(20);

    // Plan when state already has all resources (should be all Noop).
    group.bench_function("all_noop_250_resources", |b| {
        let cfg = generate_config(50, 20, 10);
        let desired = config_to_resources(&cfg).unwrap();
        let mut state = CurrentState::default();
        for r in &desired {
            let rec = guildforge_state::ResourceRecord::from_resource(r, "discord", false).unwrap();
            state.resources.insert(rec.addr.clone(), rec);
        }
        b.iter(|| {
            let planner = Planner::new();
            black_box(planner.plan(black_box(&cfg), black_box(&state)).unwrap())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_config_to_resources,
    bench_plan,
    bench_plan_with_existing_state,
);
criterion_main!(benches);
