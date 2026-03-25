use super::super::{Post, PostBlock, PostSection};
use time::macros::date;

macro_rules! overview_post {
    (
        $name:ident,
        $slug:literal,
        $title:literal,
        $excerpt:literal,
        $date:expr,
        [$($tag:literal),* $(,)?],
        [$first:literal, $second:literal, $third:literal $(,)?]
    ) => {
        pub(super) const $name: Post = Post {
            slug: $slug,
            title: $title,
            excerpt: $excerpt,
            date: $date,
            tags: &[$($tag),*],
            summary: None,
            sections: &[PostSection {
                id: "overview",
                title: "Overview",
                level: 1,
                blocks: &[
                    PostBlock::Paragraph($first),
                    PostBlock::Paragraph($second),
                    PostBlock::Paragraph($third),
                ],
            }],
        };
    };
}

overview_post!(
    ASYNC_SCHEDULER_RETROSPECTIVE,
    "async-scheduler-retrospective",
    "Async Scheduler Retrospective",
    "A retrospective on stabilising Soffio's async scheduler after migrating to a fully cooperative model.",
    date!(2022 - 12 - 12),
    ["engineering", "rust"],
    [
        "Moving to cooperative scheduling cut latency spikes by half, but revealed starvation bugs around metrics exporters.",
        "We introduced a `YieldGuard` helper that enforces fairness whenever a task touches shared I/O.",
        "Tracing spans now include logical queue depth so we can simulate contention before it happens in production.",
    ]
);

overview_post!(
    FEATURE_FLAGS_AT_SCALE,
    "feature-flags-at-scale",
    "Feature Flags at Scale",
    "How Soffio promotes experiment toggles from ad-hoc booleans to audited configuration.",
    date!(2022 - 12 - 01),
    ["infrastructure", "release"],
    [
        "Feature toggles ship with playbooks that define rollout windows, validator dashboards, and emergency reversal steps.",
        "We treat flags like schema migrations: they land with ownership, expiry dates, and automated reminders.",
        "The outcome is dull by design—the fewer surprises, the more we can trust Friday deploys.",
    ]
);

overview_post!(
    SDK_BETA_INVITE,
    "sdk-beta-invite",
    "SDK Beta Invite",
    "Announcing the Soffio SDK private beta with detailed onboarding cohorts and support expectations.",
    date!(2022 - 02 - 22),
    ["release", "community"],
    [
        "We kept the beta cohort to a dozen partner teams so feedback remains legible and we can respond within a day.",
        "Every partner receives a staging sandbox plus a weekly touchpoint that doubles as UX research.",
        "Public launch will follow once we document the migration path from our legacy CLI tooling.",
    ]
);

overview_post!(
    VISION_CALIBRATION_CHECKLIST,
    "vision-calibration-checklist",
    "Vision Calibration Checklist",
    "A behind-the-scenes guide to aligning sensor rigs for Soffio's hardware experiments.",
    date!(2022 - 02 - 22),
    ["research", "hardware"],
    [
        "Calibration begins with realigning lenses, but the non-obvious step is recording environmental conditions to reproduce the run later.",
        "We added QR-coded manifests so every cable and mount is traceable without hunting through spreadsheets.",
        "The checklist now ships with the rig so visiting teams can repeat the procedure without guidance.",
    ]
);

overview_post!(
    ENGINEERING_READING_GUIDE,
    "engineering-reading-guide",
    "Engineering Reading Guide",
    "A curated reading schedule for new Soffio teammates covering systems, teams, and tooling foundations.",
    date!(2022 - 02 - 07),
    ["learning", "community"],
    [
        "The guide alternates dense chapters with hands-on labs to keep momentum steady.",
        "We emphasise marginalia—every reader leaves questions for the next cohort, creating a living annotation trail.",
        "Graduates mentor the next group for a single session, reinforcing understanding without requiring heavy prep.",
    ]
);

overview_post!(
    FIRST_APPRENTICE_GRADUATION,
    "first-apprentice-graduation",
    "First Apprentice Graduation",
    "Celebrating Soffio's apprentice engineer who shipped a scheduler patch within eight weeks.",
    date!(2021 - 09 - 16),
    ["community"],
    [
        "Our apprenticeship pairs deep code reviews with shadowing release coordinators, so apprentices learn consequences fast.",
        "Graduates document their path, highlighting which tutorials helped and where we still lack coverage.",
        "The next cohort will experiment with pairing apprentices together to reduce silence during their first incidents.",
    ]
);

overview_post!(
    MACHINE_VISION_MYTHS,
    "machine-vision-myths",
    "Machine Vision Myths",
    "Debunking benchmark worship and outlining how Soffio evaluates perception models in production.",
    date!(2021 - 07 - 29),
    ["research", "ai"],
    [
        "Benchmarks reward lucky datasets; we instead run long-lived scenarios that match warehouse lighting and occlusion.",
        "We publish failure galleries internally so teams can see exactly what went wrong without digging through logs.",
        "The myth we keep confronting: more layers rarely beat generous labeling and disciplined retraining.",
    ]
);

overview_post!(
    SYSTEMS_PRIMER_CHAPTER,
    "systems-primer-chapter",
    "Systems Primer Chapter",
    "A sneak peek of the systems primer we hand to every Soffio hire on their first week.",
    date!(2021 - 05 - 11),
    ["learning", "engineering"],
    [
        "The chapter pairs concurrency case studies with exercises built from incidents we actually faced.",
        "Readers ship a minimal service that withstands chaos tests before they move on to the codebase.",
        "We keep the prose short but regularly update the references so the primer reflects current best practices.",
    ]
);

overview_post!(
    FOUNDATION_COHORT_FOUR,
    "foundation-cohort-four",
    "Foundation Cohort Four",
    "Insights on how we adjusted mentor ratios and project scopes for the fourth Soffio foundation cohort.",
    date!(2021 - 05 - 10),
    ["community", "release"],
    [
        "Smaller scope cards meant mentors could focus on feedback quality instead of juggling five apprentices at once.",
        "We added office-hour transcripts, giving participants a searchable log of every question.",
        "Cohorts now end with a retrospective across mentors and apprentices to decide what to keep or cut.",
    ]
);

overview_post!(
    INSTRUMENT_DISCIPLINE,
    "instrument-discipline",
    "Instrument Discipline",
    "Why Soffio invests in deliberate practice when introducing new hardware instruments.",
    date!(2021 - 04 - 16),
    ["design", "practice"],
    [
        "New instruments introduce latency every time an operator hesitates; rehearsals uncover these micro-pauses.",
        "We build muscle memory by scripting the first ten uses, then removing prompts gradually.",
        "The team now records short walkthroughs so future users inherit the same confidence curve.",
    ]
);

overview_post!(
    EXPERIMENTAL_READING_ROUNDUP,
    "experimental-reading-roundup",
    "Experimental Reading Roundup",
    "A collection of insights from Soffio's experiment with thematic reading sprints.",
    date!(2021 - 02 - 16),
    ["learning"],
    [
        "Reading sprints borrowed from book clubs but ended with demos showing how the material influenced code.",
        "Participants submit reflection snippets that we publish internally for future cohorts.",
        "The format sticks if we keep the sprint short—three weeks is the upper bound before momentum fades.",
    ]
);

overview_post!(
    FOUNDATION_COHORT_THREE,
    "foundation-cohort-three",
    "Foundation Cohort Three",
    "Lessons learned while reshaping the third Soffio foundation cohort for remote collaboration.",
    date!(2021 - 02 - 15),
    ["community"],
    [
        "Remote pairing demanded better tooling, so we standardised on a shared devcontainer image and pairing checklist.",
        "We shortened live sessions to an hour and moved deep dives into asynchronous screencasts.",
        "Graduation now includes a show-and-tell to celebrate progress and highlight open questions.",
    ]
);

overview_post!(
    COLLECTIVE_CLASS_TWO,
    "collective-class-two",
    "Collective Class Two",
    "How Soffio's collective learning track scaled to two dozen contributors without losing cohesion.",
    date!(2020 - 09 - 24),
    ["community", "learning"],
    [
        "We mapped every exercise to a real subsystem so results could ship instead of languishing in side projects.",
        "Facilitators rotated weekly, offering everyone a chance to lead while keeping responsibilities light.",
        "A shared glossary helped newcomers catch up without derailing sessions with definitions.",
    ]
);

overview_post!(
    LANGUAGE_HABITS,
    "language-habits",
    "Language Habits",
    "Guidelines Soffio writers follow to keep documentation focused and culturally neutral.",
    date!(2020 - 05 - 23),
    ["communication"],
    [
        "We avoid region-specific idioms and acronyms unless the doc provides definitions inline.",
        "Writers read drafts aloud—a quick way to catch meandering sentences and implicit assumptions.",
        "The style guide is versioned alongside the docs so edits travel together in the same pull request.",
    ]
);

overview_post!(
    ONE_ON_ONE_COACHING,
    "one-on-one-coaching",
    "One-on-One Coaching",
    "Refreshing Soffio's coaching program so mentors can sustain deep engagements without burning out.",
    date!(2020 - 04 - 30),
    ["community", "learning"],
    [
        "We guard mentor calendars by batching sessions into dedicated afternoons and protecting focus blocks elsewhere.",
        "Coaching summaries now live in a private repo, giving continuity between sessions while respecting privacy.",
        "Participants commit to a demo at the end of the track, making growth measurable and concrete.",
    ]
);

overview_post!(
    ENTRY_LEVEL_LAUNCH,
    "entry-level-launch",
    "Entry-Level Launch",
    "Announcing Soffio's entry-level program with a curriculum focused on toolsmithing and collaboration.",
    date!(2020 - 03 - 26),
    ["community", "release"],
    [
        "The launch cohort learns by shipping internal tooling that removes friction for senior engineers.",
        "Every participant rotates through support duty to build empathy with the users they just unblocked.",
        "We measure success not by velocity but by the quality of postmortems and follow-up fixes.",
    ]
);
