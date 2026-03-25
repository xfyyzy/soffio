use super::Post;

#[path = "data/longform.rs"]
mod longform;
#[path = "data/overview.rs"]
mod overview;

pub static POSTS: [Post; 19] = [
    longform::INCREMENTAL_BUILD_PIPELINE,
    longform::OBSERVABILITY_CONTROL_PLANE_ROLLOUT,
    longform::EDGE_CACHE_WASM_PROFILING,
    overview::ASYNC_SCHEDULER_RETROSPECTIVE,
    overview::FEATURE_FLAGS_AT_SCALE,
    overview::SDK_BETA_INVITE,
    overview::VISION_CALIBRATION_CHECKLIST,
    overview::ENGINEERING_READING_GUIDE,
    overview::FIRST_APPRENTICE_GRADUATION,
    overview::MACHINE_VISION_MYTHS,
    overview::SYSTEMS_PRIMER_CHAPTER,
    overview::FOUNDATION_COHORT_FOUR,
    overview::INSTRUMENT_DISCIPLINE,
    overview::EXPERIMENTAL_READING_ROUNDUP,
    overview::FOUNDATION_COHORT_THREE,
    overview::COLLECTIVE_CLASS_TWO,
    overview::LANGUAGE_HABITS,
    overview::ONE_ON_ONE_COACHING,
    overview::ENTRY_LEVEL_LAUNCH,
];
