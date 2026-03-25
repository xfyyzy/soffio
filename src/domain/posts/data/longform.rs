use super::super::{Post, PostBlock, PostSection};
use time::macros::date;

pub(super) const INCREMENTAL_BUILD_PIPELINE: Post = Post {
    slug: "incremental-build-pipeline",
    title: "Incremental Build Pipeline in Rust",
    excerpt: "How we rebuilt Soffio's Rust workspace pipeline to deliver artifacts every five minutes without sacrificing determinism.",
    date: date!(2025 - 05 - 12),
    tags: &["engineering", "build"],
    summary: Some(&[
        "Crate-level DAG scheduling slashes end-to-end build latency to under five minutes per change.",
        "Remote fingerprints and targeted linting stop unnecessary workspace rebuilds.",
        "Trace-enriched metadata lets engineers replay failures locally without waiting on CI.",
    ]),
    sections: &[
        PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Soffio's legacy mono-build consumed an hour per change even when only a leaf crate changed. We replaced it with a directed acyclic graph generated from the workspace manifest, so each crate now advertises its edges explicitly.",
                ),
                PostBlock::Paragraph(
                    "Schedulers assign nodes to build agents based on cache affinity. Each node materialises a fingerprint derived from `Cargo.lock`, environment variables, and compiler flags, which keeps the graph deterministic across builds.",
                ),
            ],
        },
        PostSection {
            id: "architecture",
            title: "Pipeline Architecture",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "We expose the pipeline graph through a small Rust orchestrator. It inspects the workspace, computes the ready set, and only queues crates that need work. Everything else is a cache hit and exits early.",
                ),
                PostBlock::Code {
                    language: "rust",
                    code: r#"fn enqueue(node: WorkspaceNode) {
    if cache.hit(&node.fingerprint) {
        return;
    }
    scheduler.spawn(node);
}"#,
                },
                PostBlock::Paragraph(
                    "Linkers, lints, and docs run in dedicated lanes. When a node builds successfully we broadcast its artifact location and fingerprint through Redis so dependent crates can resume without polling.",
                ),
            ],
        },
        PostSection {
            id: "validation",
            title: "Validation & Observability",
            level: 1,
            blocks: &[
                PostBlock::List(&[
                    "Integration tests publish per-crate timings to Prometheus, letting us compare hot spots between runs.",
                    "Every build stores its fingerprint metadata in S3, so rerunning `soffio-pipeline replay --build <id>` fully reconstructs the environment.",
                ]),
                PostBlock::Paragraph(
                    "Within two weeks p95 build time fell from 47 minutes to 4.8 minutes. More importantly, developers finally trust the cache because they can inspect when and why it missed.",
                ),
            ],
        },
        PostSection {
            id: "next-steps",
            title: "Next Steps",
            level: 2,
            blocks: &[
                PostBlock::Paragraph(
                    "We are now targeting dependency-aware rollbacks and live dashboards showing which crates consume the cache budget.",
                ),
                PostBlock::List(&[
                    "Integrate design-system change detection so doc builds remain optional when untouched.",
                    "Ship per-crate success dashboards for the release team, including alerts on cache eviction spikes.",
                ]),
            ],
        },
    ],
};

pub(super) const OBSERVABILITY_CONTROL_PLANE_ROLLOUT: Post = Post {
    slug: "observability-control-plane",
    title: "Observability Control Plane Rollout",
    excerpt: "Refactoring metrics ingestion into a control plane that keeps dashboards within 5s of reality even during deploy storms.",
    date: date!(2024 - 05 - 03),
    tags: &["observability", "platform"],
    summary: Some(&[
        "Control plane validates metric schemas before samples reach storage.",
        "CRDT-backed configuration keeps remote agents consistent during partitions.",
        "Freshness p99 improved from 41s to 4.7s after rollout.",
    ]),
    sections: &[
        PostSection {
            id: "motivation",
            title: "Why a Control Plane?",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Our previous observability stack pushed Prometheus scrape pressure straight to storage nodes, causing tail latency and dropped samples whenever we deployed a new service.",
                ),
                PostBlock::Paragraph(
                    "We needed a single source of truth for metric schemas, retention rules, and remote agents. The control plane now validates new metrics before any data plane node accepts them.",
                ),
            ],
        },
        PostSection {
            id: "design",
            title: "Control Plane Design",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "A Rust gRPC service exposes a declarative API. Teams submit manifests describing metrics and expected cardinality. The control plane diff-checks these manifests and writes the merged state into a CRDT store.",
                ),
                PostBlock::Code {
                    language: "yaml",
                    code: r#"metric:
  name: http_request_duration_seconds
  type: histogram
  expected_cardinality: 400
ingestion:
  tenant: soffio-apps
  retention_days: 14
  routes:
    - regex: "^api-"
      target_pool: edge-us"#,
                },
                PostBlock::Paragraph(
                    "Agents subscribe to the CRDT via WebSocket. When the control plane validates a config, it pushes a signed snapshot to every region atomically. Agents fall back to the last good snapshot if they detect divergence.",
                ),
            ],
        },
        PostSection {
            id: "data-plane",
            title: "Data Plane Execution",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Raw samples land in an Apache Arrow buffer before being flushed to Parquet files. Because the schema matches Grafana's query schema, dashboards and ad-hoc traces finally agree.",
                ),
                PostBlock::List(&[
                    "Prometheus remote-write adapter streams directly into Parquet segmented by tenant and metric label set.",
                    "BigQuery ingestion jobs now run off the same dataset, eliminating the stale copy we previously maintained.",
                ]),
            ],
        },
        PostSection {
            id: "results",
            title: "Results",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Chaos drills show p99 scrape freshness dropped from 41s to 4.7s during deploy storms. Alert fatigue decreased once we tuned the policy centrally.",
                ),
                PostBlock::Paragraph(
                    "Engineers now trust the dashboards because they can audit every metric via the control plane manifest.",
                ),
            ],
        },
        PostSection {
            id: "next-steps",
            title: "Next Steps",
            level: 2,
            blocks: &[PostBlock::Paragraph(
                "We plan to expose the manifest API externally so teams can self-serve onboarding and implement anomaly detection over the Arrow dataset.",
            )],
        },
    ],
};

pub(super) const EDGE_CACHE_WASM_PROFILING: Post = Post {
    slug: "edge-cache-wasm-profiling",
    title: "边缘缓存的 Wasm 画像实践",
    excerpt: "借助 WebAssembly sidecar 在生产环境诊断 Soffio 边缘缓存的性能瓶颈。",
    date: date!(2023 - 12 - 23),
    tags: &["performance", "wasm"],
    summary: Some(&[
        "Proxy-Wasm sidecar 不必重启守护进程即可更新采样逻辑。",
        "合成回放帮助比较新旧淘汰策略的命中率差异。",
        "上线后 95% 延迟下降 18%，缓存命中率提升 6 个百分点。",
    ]),
    sections: &[
        PostSection {
            id: "背景",
            title: "背景",
            level: 1,
            blocks: &[PostBlock::Paragraph(
                "Soffio 的边缘缓存需要在高流量下快速定位性能退化。我们采用 Proxy-Wasm 在请求生命周期的关键点插桩，收集细粒度指标。",
            )],
        },
        PostSection {
            id: "实现",
            title: "实现细节",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "每个工作进程旁挂一个 Wasm sidecar，负责记录命中/未命中的时间桶，并将数据批量回传控制平面。",
                ),
                PostBlock::Code {
                    language: "wasm",
                    code: r#"(module
  (import "env" "log_timing" (func $log (param i32)))
  (func (export "on_tick") (param $delta i32)
    call $log))"#,
                },
                PostBlock::Paragraph(
                    "我们对热门键进行采样，并在合成环境重放流量，比较不同淘汰策略的命中率差异。",
                ),
            ],
        },
        PostSection {
            id: "结果",
            title: "结果与展望",
            level: 1,
            blocks: &[PostBlock::Paragraph(
                "变更发布后 95% 延迟下降 18%，缓存命中率提升 6 个百分点。下一步计划将 Wasm 模块与控制平面的模板系统打通，实现按租户自定义采样。",
            )],
        },
    ],
};
