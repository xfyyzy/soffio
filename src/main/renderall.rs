use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use apalis::prelude::Data;
use futures::stream::TryStreamExt;
use soffio::{
    application::error::AppError,
    application::jobs::JobWorkerContext,
    application::render::{
        RenderPageJobPayload, RenderPostJobPayload, process_render_page_job,
        process_render_post_job,
    },
    config,
    domain::entities::{PageRecord, PostRecord},
};
use tracing::info;

use crate::serve::{build_application_context, init_repositories};

pub(super) async fn run_renderall(
    settings: config::Settings,
    args: config::RenderAllArgs,
) -> Result<(), AppError> {
    let (http_repositories, job_repositories) = init_repositories(&settings).await?;
    let app = build_application_context(http_repositories, job_repositories, &settings)?;
    let job_context = app.job_context;

    let filter_specified = args.posts || args.pages;
    let render_posts = if filter_specified { args.posts } else { true };
    let render_pages = if filter_specified { args.pages } else { true };

    if !render_posts && !render_pages {
        return Err(AppError::validation(
            "renderall requires at least one of --posts or --pages",
        ));
    }

    let concurrency = args.concurrency.clamp(1, 32);

    info!(
        target = "soffio::renderall",
        concurrency,
        posts = render_posts,
        pages = render_pages,
        "Starting renderall"
    );

    if render_posts {
        render_all_posts(&job_context, concurrency).await?;
    }

    if render_pages {
        render_all_pages(&job_context, concurrency).await?;
    }

    Ok(())
}

async fn render_all_posts(ctx: &JobWorkerContext, concurrency: usize) -> Result<(), AppError> {
    let total = Arc::new(AtomicUsize::new(0));
    let worker_ctx = ctx.clone();
    let total_handle = total.clone();

    ctx.repositories
        .stream_all_posts()
        .map_err(|err| AppError::unexpected(err.to_string()))
        .try_for_each_concurrent(Some(concurrency), move |post| {
            let ctx = worker_ctx.clone();
            let counter = total_handle.clone();
            async move {
                render_post(&ctx, post).await?;
                counter.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        })
        .await?;

    let count = total.load(Ordering::Relaxed);
    info!(
        target = "soffio::renderall",
        posts = count,
        "Rendered all posts"
    );
    Ok(())
}

async fn render_all_pages(ctx: &JobWorkerContext, concurrency: usize) -> Result<(), AppError> {
    let total = Arc::new(AtomicUsize::new(0));
    let worker_ctx = ctx.clone();
    let total_handle = total.clone();

    ctx.repositories
        .stream_all_pages()
        .map_err(|err| AppError::unexpected(err.to_string()))
        .try_for_each_concurrent(Some(concurrency), move |page| {
            let ctx = worker_ctx.clone();
            let counter = total_handle.clone();
            async move {
                render_page(&ctx, page).await?;
                counter.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        })
        .await?;

    let count = total.load(Ordering::Relaxed);
    info!(
        target = "soffio::renderall",
        pages = count,
        "Rendered all pages"
    );
    Ok(())
}

async fn render_post(ctx: &JobWorkerContext, post: PostRecord) -> Result<(), AppError> {
    process_render_post_job(
        RenderPostJobPayload {
            slug: post.slug.clone(),
            body_markdown: post.body_markdown.clone(),
            summary_markdown: post.summary_markdown.clone(),
        },
        Data::new(ctx.clone()),
    )
    .await
    .map_err(|err| AppError::unexpected(format!("render `{}` failed: {err}", post.slug)))?;

    Ok(())
}

async fn render_page(ctx: &JobWorkerContext, page: PageRecord) -> Result<(), AppError> {
    process_render_page_job(
        RenderPageJobPayload {
            slug: page.slug.clone(),
            markdown: page.body_markdown.clone(),
        },
        Data::new(ctx.clone()),
    )
    .await
    .map_err(|err| AppError::unexpected(format!("render page `{}` failed: {err}", page.slug)))?;

    Ok(())
}
