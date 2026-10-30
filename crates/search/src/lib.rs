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
        let index = self
            .client
            .create_index(&self.index_name, Some("id"))
            .await;
        let _ = index; // exists ok
        let index = self.client.index(&self.index_name);
        index
            .set_filterable_attributes(["owner_user_id", "category_id", "tags"])
            .await
            .context("filterable attrs")?;
        index
            .set_searchable_attributes(["title", "url", "description", "notes", "tags"])
            .await
            .context("searchable attrs")?;
        Ok(())
    }

    pub async fn upsert_bookmark(&self, owner: Uuid, view: &BookmarkView) -> anyhow::Result<()> {
        if view.is_vault {
            return Ok(());
        }
        let doc = BookmarkDoc {
            id: view.id.to_string(),
            owner_user_id: owner.to_string(),
            title: view.title.clone(),
            url: view.url.clone(),
            description: view.description.clone().unwrap_or_default(),
            notes: view.notes.clone().unwrap_or_default(),
            tags: view.tags.clone(),
            category_id: view.category_id.map(|c| c.to_string()),
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
        let _ = task.wait_for_completion(&self.client, None, None).await;
        Ok(())
    }

    pub async fn delete_bookmark(&self, id: Uuid) -> anyhow::Result<()> {
        let task = self
            .client
            .index(&self.index_name)
            .delete_document(id.to_string())
            .await
            .context("meili delete")?;
        let _ = task.wait_for_completion(&self.client, None, None).await;
        Ok(())
    }

    pub async fn search(
        &self,
        owner: Uuid,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<BookmarkDoc>> {
        let index = self.client.index(&self.index_name);
        let filter = format!("owner_user_id = \"{owner}\"");
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
