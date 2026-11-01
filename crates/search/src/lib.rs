use anyhow::Context;
use linkdean_core::BookmarkView;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::search::SearchQuery;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct SearchClient {
    client: Client,
    index_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkDoc {
    pub id: String,
    pub owner_user_id: String,
    pub title: String,
    pub url: String,
    pub description: String,
    pub notes: String,
    pub tags: Vec<String>,
    pub category_id: Option<String>,
    #[serde(default)]
    pub category_ids: Vec<String>,
    #[serde(default)]
    pub favicon_url: Option<String>,
    #[serde(default)]
    pub preview_image_url: Option<String>,
    #[serde(default)]
    pub has_preview: bool,
}

impl SearchClient {
    pub fn new(url: &str, key: &str) -> anyhow::Result<Self> {
        let client = Client::new(url, Some(key)).context("meilisearch client")?;
        Ok(Self {
            client,
            index_name: "bookmarks".into(),
        })
    }

    pub async fn ensure_index(&self) -> anyhow::Result<()> {
        let index = self.client.create_index(&self.index_name, Some("id")).await;
        let _ = index; // exists ok
        let index = self.client.index(&self.index_name);
        index
            .set_filterable_attributes([
                "owner_user_id",
                "category_id",
                "category_ids",
                "tags",
            ])
            .await
            .context("filterable attrs")?;
        index
            .set_searchable_attributes(["title", "url", "description", "notes", "tags"])
            .await
            .context("searchable attrs")?;
        Ok(())
    }

    /// Index using the bookmark's real `owner_user_id` (not the actor who edited).
    pub async fn upsert_bookmark(&self, view: &BookmarkView) -> anyhow::Result<()> {
        if view.is_vault {
            return Ok(());
        }
        let category_ids: Vec<String> = view.categories.iter().map(|c| c.id.to_string()).collect();
        let doc = BookmarkDoc {
            id: view.id.to_string(),
            owner_user_id: view.owner_user_id.to_string(),
            title: view.title.clone(),
            url: view.url.clone(),
            description: view.description.clone().unwrap_or_default(),
            notes: view.notes.clone().unwrap_or_default(),
            tags: view.tags.clone(),
            category_id: view.category_id.map(|c| c.to_string()),
            category_ids,
            favicon_url: view.favicon_url.clone(),
            preview_image_url: view.preview_image_url.clone(),
            has_preview: view.has_preview,
        };
        let task = self
            .client
            .index(&self.index_name)
            .add_or_replace(&[doc], None)
            .await
            .context("meili upsert")?;
        if let Err(e) = task.wait_for_completion(&self.client, None, None).await {
            tracing::warn!(bookmark_id = %view.id, error = %e, "meili upsert wait failed");
        }
        Ok(())
    }

    pub async fn delete_bookmark(&self, id: Uuid) -> anyhow::Result<()> {
        let task = self
            .client
            .index(&self.index_name)
            .delete_document(id.to_string())
            .await
            .context("meili delete")?;
        if let Err(e) = task.wait_for_completion(&self.client, None, None).await {
            tracing::warn!(bookmark_id = %id, error = %e, "meili delete wait failed");
        }
        Ok(())
    }

    /// Drop every document so a full reindex starts clean (stale docs gone).
    pub async fn clear_index(&self) -> anyhow::Result<()> {
        let task = self
            .client
            .index(&self.index_name)
            .delete_all_documents()
            .await
            .context("meili delete all")?;
        task.wait_for_completion(&self.client, None, None)
            .await
            .context("meili delete all wait")?;
        Ok(())
    }

    /// Search bookmarks owned by `user` or filed under any of `readable_category_ids`.
    pub async fn search(
        &self,
        user: Uuid,
        readable_category_ids: &[Uuid],
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<BookmarkDoc>> {
        let index = self.client.index(&self.index_name);
        let filter = meili_visibility_filter(user, readable_category_ids);
        let results = SearchQuery::new(&index)
            .with_query(query)
            .with_filter(&filter)
            .with_limit(limit)
            .execute::<BookmarkDoc>()
            .await
            .context("meili search")?;
        Ok(results.hits.into_iter().map(|h| h.result).collect())
    }
}

fn meili_visibility_filter(user: Uuid, readable_category_ids: &[Uuid]) -> String {
    let owner = format!("owner_user_id = \"{user}\"");
    if readable_category_ids.is_empty() {
        return owner;
    }
    let quoted: Vec<String> = readable_category_ids
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect();
    let list = quoted.join(", ");
    // Match primary category_id (legacy docs) or category_ids array (current upserts).
    format!("{owner} OR category_id IN [{list}] OR category_ids IN [{list}]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_owner_only_when_no_cats() {
        let uid = Uuid::nil();
        assert_eq!(
            meili_visibility_filter(uid, &[]),
            format!("owner_user_id = \"{uid}\"")
        );
    }

    #[test]
    fn filter_includes_category_ors() {
        let uid = Uuid::nil();
        let c1 = Uuid::from_u128(1);
        let c2 = Uuid::from_u128(2);
        let f = meili_visibility_filter(uid, &[c1, c2]);
        assert!(f.contains("owner_user_id"));
        assert!(f.contains(&format!("\"{c1}\"")));
        assert!(f.contains("category_ids IN"));
    }
}
