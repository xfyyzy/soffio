use super::*;

#[path = "api_end_to_end/cleanup.rs"]
mod cleanup;
#[path = "api_end_to_end/navigation.rs"]
mod navigation;
#[path = "api_end_to_end/pages.rs"]
mod pages;
#[path = "api_end_to_end/posts.rs"]
mod posts;
#[path = "api_end_to_end/settings_jobs_audit.rs"]
mod settings_jobs_audit;
#[path = "api_end_to_end/tags.rs"]
mod tags;
#[path = "api_end_to_end/uploads.rs"]
mod uploads;

pub(super) struct LiveApiContext<'a> {
    pub(super) client: &'a Client,
    pub(super) base: &'a str,
    pub(super) config: &'a SeedConfig,
    pub(super) suffix: &'a str,
}

pub(super) struct TagFixture {
    pub(super) id: String,
}

pub(super) struct PostFixture {
    pub(super) id: String,
}

pub(super) struct PageFixture {
    pub(super) id: String,
}

pub(super) struct PageBootstrapFixture {
    pub(super) page: PageFixture,
    pub(super) nav_id_1: String,
}

pub(super) struct NavigationFixture {
    pub(super) nav_id_2: String,
}

pub(super) struct UploadFixture {
    pub(super) id: String,
}

#[tokio::test]
#[ignore]
async fn live_api_end_to_end() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();
    let suffix = current_suffix();
    let ctx = LiveApiContext {
        client: &client,
        base: &base,
        config: &config,
        suffix: &suffix,
    };

    let tag = tags::exercise(&ctx).await?;
    let post = posts::exercise(&ctx, &tag).await?;
    let page_bootstrap = pages::exercise(&ctx).await?;
    let navigation = navigation::exercise(&ctx).await?;
    let upload = uploads::exercise(&ctx).await?;
    settings_jobs_audit::exercise(&ctx).await?;
    cleanup::exercise(&ctx, &tag, &post, &page_bootstrap, &navigation, &upload).await?;

    Ok(())
}
