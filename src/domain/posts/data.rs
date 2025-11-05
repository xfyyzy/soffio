use super::{Post, PostBlock, PostSection};
use time::macros::date;

pub static POSTS: [Post; 19] = [
    Post {
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
    },
    Post {
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
    },
    Post {
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
    },
    Post {
        slug: "async-scheduler-retrospective",
        title: "Async Scheduler Retrospective",
        excerpt: "A retrospective on stabilising Soffio's async scheduler after migrating to a fully cooperative model.",
        date: date!(2022 - 12 - 12),
        tags: &["engineering", "rust"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Moving to cooperative scheduling cut latency spikes by half, but revealed starvation bugs around metrics exporters.",
                ),
                PostBlock::Paragraph(
                    "We introduced a `YieldGuard` helper that enforces fairness whenever a task touches shared I/O.",
                ),
                PostBlock::Paragraph(
                    "Tracing spans now include logical queue depth so we can simulate contention before it happens in production.",
                ),
            ],
        }],
    },
    Post {
        slug: "feature-flags-at-scale",
        title: "Feature Flags at Scale",
        excerpt: "How Soffio promotes experiment toggles from ad-hoc booleans to audited configuration.",
        date: date!(2022 - 12 - 01),
        tags: &["infrastructure", "release"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Feature toggles ship with playbooks that define rollout windows, validator dashboards, and emergency reversal steps.",
                ),
                PostBlock::Paragraph(
                    "We treat flags like schema migrations: they land with ownership, expiry dates, and automated reminders.",
                ),
                PostBlock::Paragraph(
                    "The outcome is dull by design—the fewer surprises, the more we can trust Friday deploys.",
                ),
            ],
        }],
    },
    Post {
        slug: "sdk-beta-invite",
        title: "SDK Beta Invite",
        excerpt: "Announcing the Soffio SDK private beta with detailed onboarding cohorts and support expectations.",
        date: date!(2022 - 02 - 22),
        tags: &["release", "community"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "We kept the beta cohort to a dozen partner teams so feedback remains legible and we can respond within a day.",
                ),
                PostBlock::Paragraph(
                    "Every partner receives a staging sandbox plus a weekly touchpoint that doubles as UX research.",
                ),
                PostBlock::Paragraph(
                    "Public launch will follow once we document the migration path from our legacy CLI tooling.",
                ),
            ],
        }],
    },
    Post {
        slug: "vision-calibration-checklist",
        title: "Vision Calibration Checklist",
        excerpt: "A behind-the-scenes guide to aligning sensor rigs for Soffio's hardware experiments.",
        date: date!(2022 - 02 - 22),
        tags: &["research", "hardware"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Calibration begins with realigning lenses, but the non-obvious step is recording environmental conditions to reproduce the run later.",
                ),
                PostBlock::Paragraph(
                    "We added QR-coded manifests so every cable and mount is traceable without hunting through spreadsheets.",
                ),
                PostBlock::Paragraph(
                    "The checklist now ships with the rig so visiting teams can repeat the procedure without guidance.",
                ),
            ],
        }],
    },
    Post {
        slug: "engineering-reading-guide",
        title: "Engineering Reading Guide",
        excerpt: "A curated reading schedule for new Soffio teammates covering systems, teams, and tooling foundations.",
        date: date!(2022 - 02 - 07),
        tags: &["learning", "community"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "The guide alternates dense chapters with hands-on labs to keep momentum steady.",
                ),
                PostBlock::Paragraph(
                    "We emphasise marginalia—every reader leaves questions for the next cohort, creating a living annotation trail.",
                ),
                PostBlock::Paragraph(
                    "Graduates mentor the next group for a single session, reinforcing understanding without requiring heavy prep.",
                ),
            ],
        }],
    },
    Post {
        slug: "first-apprentice-graduation",
        title: "First Apprentice Graduation",
        excerpt: "Celebrating Soffio's apprentice engineer who shipped a scheduler patch within eight weeks.",
        date: date!(2021 - 09 - 16),
        tags: &["community"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Our apprenticeship pairs deep code reviews with shadowing release coordinators, so apprentices learn consequences fast.",
                ),
                PostBlock::Paragraph(
                    "Graduates document their path, highlighting which tutorials helped and where we still lack coverage.",
                ),
                PostBlock::Paragraph(
                    "The next cohort will experiment with pairing apprentices together to reduce silence during their first incidents.",
                ),
            ],
        }],
    },
    Post {
        slug: "machine-vision-myths",
        title: "Machine Vision Myths",
        excerpt: "Debunking benchmark worship and outlining how Soffio evaluates perception models in production.",
        date: date!(2021 - 07 - 29),
        tags: &["research", "ai"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Benchmarks reward lucky datasets; we instead run long-lived scenarios that match warehouse lighting and occlusion.",
                ),
                PostBlock::Paragraph(
                    "We publish failure galleries internally so teams can see exactly what went wrong without digging through logs.",
                ),
                PostBlock::Paragraph(
                    "The myth we keep confronting: more layers rarely beat generous labeling and disciplined retraining.",
                ),
            ],
        }],
    },
    Post {
        slug: "systems-primer-chapter",
        title: "Systems Primer Chapter",
        excerpt: "A sneak peek of the systems primer we hand to every Soffio hire on their first week.",
        date: date!(2021 - 05 - 11),
        tags: &["learning", "engineering"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "The chapter pairs concurrency case studies with exercises built from incidents we actually faced.",
                ),
                PostBlock::Paragraph(
                    "Readers ship a minimal service that withstands chaos tests before they move on to the codebase.",
                ),
                PostBlock::Paragraph(
                    "We keep the prose short but regularly update the references so the primer reflects current best practices.",
                ),
            ],
        }],
    },
    Post {
        slug: "foundation-cohort-four",
        title: "Foundation Cohort Four",
        excerpt: "Insights on how we adjusted mentor ratios and project scopes for the fourth Soffio foundation cohort.",
        date: date!(2021 - 05 - 10),
        tags: &["community", "release"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Smaller scope cards meant mentors could focus on feedback quality instead of juggling five apprentices at once.",
                ),
                PostBlock::Paragraph(
                    "We added office-hour transcripts, giving participants a searchable log of every question.",
                ),
                PostBlock::Paragraph(
                    "Cohorts now end with a retrospective across mentors and apprentices to decide what to keep or cut.",
                ),
            ],
        }],
    },
    Post {
        slug: "instrument-discipline",
        title: "Instrument Discipline",
        excerpt: "Why Soffio invests in deliberate practice when introducing new hardware instruments.",
        date: date!(2021 - 04 - 16),
        tags: &["design", "practice"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "New instruments introduce latency every time an operator hesitates; rehearsals uncover these micro-pauses.",
                ),
                PostBlock::Paragraph(
                    "We build muscle memory by scripting the first ten uses, then removing prompts gradually.",
                ),
                PostBlock::Paragraph(
                    "The team now records short walkthroughs so future users inherit the same confidence curve.",
                ),
            ],
        }],
    },
    Post {
        slug: "experimental-reading-roundup",
        title: "Experimental Reading Roundup",
        excerpt: "A collection of insights from Soffio's experiment with thematic reading sprints.",
        date: date!(2021 - 02 - 16),
        tags: &["learning"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Reading sprints borrowed from book clubs but ended with demos showing how the material influenced code.",
                ),
                PostBlock::Paragraph(
                    "Participants submit reflection snippets that we publish internally for future cohorts.",
                ),
                PostBlock::Paragraph(
                    "The format sticks if we keep the sprint short—three weeks is the upper bound before momentum fades.",
                ),
            ],
        }],
    },
    Post {
        slug: "foundation-cohort-three",
        title: "Foundation Cohort Three",
        excerpt: "Lessons learned while reshaping the third Soffio foundation cohort for remote collaboration.",
        date: date!(2021 - 02 - 15),
        tags: &["community"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "Remote pairing demanded better tooling, so we standardised on a shared devcontainer image and pairing checklist.",
                ),
                PostBlock::Paragraph(
                    "We shortened live sessions to an hour and moved deep dives into asynchronous screencasts.",
                ),
                PostBlock::Paragraph(
                    "Graduation now includes a show-and-tell to celebrate progress and highlight open questions.",
                ),
            ],
        }],
    },
    Post {
        slug: "collective-class-two",
        title: "Collective Class Two",
        excerpt: "How Soffio's collective learning track scaled to two dozen contributors without losing cohesion.",
        date: date!(2020 - 09 - 24),
        tags: &["community", "learning"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "We mapped every exercise to a real subsystem so results could ship instead of languishing in side projects.",
                ),
                PostBlock::Paragraph(
                    "Facilitators rotated weekly, offering everyone a chance to lead while keeping responsibilities light.",
                ),
                PostBlock::Paragraph(
                    "A shared glossary helped newcomers catch up without derailing sessions with definitions.",
                ),
            ],
        }],
    },
    Post {
        slug: "language-habits",
        title: "Language Habits",
        excerpt: "Guidelines Soffio writers follow to keep documentation focused and culturally neutral.",
        date: date!(2020 - 05 - 23),
        tags: &["communication"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "We avoid region-specific idioms and acronyms unless the doc provides definitions inline.",
                ),
                PostBlock::Paragraph(
                    "Writers read drafts aloud—a quick way to catch meandering sentences and implicit assumptions.",
                ),
                PostBlock::Paragraph(
                    "The style guide is versioned alongside the docs so edits travel together in the same pull request.",
                ),
            ],
        }],
    },
    Post {
        slug: "one-on-one-coaching",
        title: "One-on-One Coaching",
        excerpt: "Refreshing Soffio's coaching program so mentors can sustain deep engagements without burning out.",
        date: date!(2020 - 04 - 30),
        tags: &["community", "learning"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "We guard mentor calendars by batching sessions into dedicated afternoons and protecting focus blocks elsewhere.",
                ),
                PostBlock::Paragraph(
                    "Coaching summaries now live in a private repo, giving continuity between sessions while respecting privacy.",
                ),
                PostBlock::Paragraph(
                    "Participants commit to a demo at the end of the track, making growth measurable and concrete.",
                ),
            ],
        }],
    },
    Post {
        slug: "entry-level-launch",
        title: "Entry-Level Launch",
        excerpt: "Announcing Soffio's entry-level program with a curriculum focused on toolsmithing and collaboration.",
        date: date!(2020 - 03 - 26),
        tags: &["community", "release"],
        summary: None,
        sections: &[PostSection {
            id: "overview",
            title: "Overview",
            level: 1,
            blocks: &[
                PostBlock::Paragraph(
                    "The launch cohort learns by shipping internal tooling that removes friction for senior engineers.",
                ),
                PostBlock::Paragraph(
                    "Every participant rotates through support duty to build empathy with the users they just unblocked.",
                ),
                PostBlock::Paragraph(
                    "We measure success not by velocity but by the quality of postmortems and follow-up fixes.",
                ),
            ],
        }],
    },
];
