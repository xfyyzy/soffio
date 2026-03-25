use super::*;

#[async_trait]
impl SectionsRepo for StaticContentRepo {
    async fn list_sections(&self, post_id: Uuid) -> Result<Vec<PostSectionRecord>, RepoError> {
        let post = self
            .all_posts()
            .into_iter()
            .find(|post| Self::post_uuid(post.slug) == post_id)
            .expect("post exists");
        Ok(Self::sections_for(post))
    }
}
