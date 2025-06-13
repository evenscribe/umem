use anyhow::Result;
use qdrant_client::{
    Qdrant,
    qdrant::{
        CreateCollectionBuilder, Filter, GetCollectionInfoResponse, PointStruct,
        SearchPointsBuilder, SearchResponse, UpsertPointsBuilder,
    },
};
use std::sync::Arc;

pub struct QdrantClientWrapper {
    // TODO: Build a connection pool for QdrantClient
    client: Arc<Qdrant>,
}

impl Clone for QdrantClientWrapper {
    fn clone(&self) -> Self {
        QdrantClientWrapper {
            client: Arc::clone(&self.client),
        }
    }
}

impl QdrantClientWrapper {
    pub fn new(url: &str, api_key: &str) -> Result<Self> {
        let client = Qdrant::from_url(url)
            .api_key(api_key)
            .skip_compatibility_check()
            .build()?;
        Ok(QdrantClientWrapper {
            client: Arc::new(client),
        })
    }

    pub async fn create_collection(
        &self,
        collection_builder: CreateCollectionBuilder,
    ) -> Result<()> {
        self.client.create_collection(collection_builder).await?;
        Ok(())
    }

    pub async fn delete_collection(&self, collection_name: &str) -> Result<()> {
        self.client.delete_collection(collection_name).await?;
        Ok(())
    }

    pub async fn collection_metadata(
        &self,
        collection_name: &str,
    ) -> Result<GetCollectionInfoResponse> {
        Ok(self.client.collection_info(collection_name).await?)
    }

    pub async fn insert_embedding(&self, collection_name: &str, point: PointStruct) -> Result<()> {
        self.client
            .upsert_points(UpsertPointsBuilder::new(collection_name, [point]))
            .await?;
        Ok(())
    }

    pub async fn insert_embeddings_bulk<I>(&self, collection_name: &str, points: I) -> Result<()>
    where
        I: IntoIterator<Item = PointStruct>,
    {
        self.client
            .upsert_points(UpsertPointsBuilder::new(
                collection_name,
                points.into_iter().collect::<Vec<PointStruct>>(),
            ))
            .await?;
        Ok(())
    }

    pub async fn search_with_vector(
        &self,
        collection_name: &str,
        vector: Vec<f32>,
        limit: Option<u64>,
    ) -> Result<SearchResponse> {
        let limit = limit.unwrap_or(10);
        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(collection_name, vector, limit).with_payload(true),
            )
            .await?;

        Ok(search_result)
    }

    pub async fn filtered_search_with_vector(
        &self,
        collection_name: &str,
        vector: Vec<f32>,
        filter: Filter,
        limit: Option<u64>,
    ) -> Result<SearchResponse> {
        let limit = limit.unwrap_or(10);
        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(collection_name, vector, limit)
                    .with_payload(true)
                    .filter(filter),
            )
            .await?;

        Ok(search_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use qdrant_client::{
        Payload,
        qdrant::{
            Condition, CreateCollectionBuilder, Distance, Filter, PointId, PointStruct,
            VectorParamsBuilder,
        },
    };
    use serde_json::json;
    use std::env;

    async fn get_test_client() -> Result<QdrantClientWrapper> {
        let url = env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".to_string());
        QdrantClientWrapper::new(&url, "YOUR_API_KEY")
    }

    async fn create_test_collection(client: &QdrantClientWrapper, name: &str) -> Result<()> {
        let vector_size = 10;

        let _ = client.delete_collection(name).await;

        client
            .create_collection(
                CreateCollectionBuilder::new(name)
                    .vectors_config(VectorParamsBuilder::new(vector_size, Distance::Cosine)),
            )
            .await
    }

    #[tokio::test]
    async fn test_create_and_delete_collection() -> Result<()> {
        let client = get_test_client().await?;
        let collection_name = "test_create_delete";

        create_test_collection(&client, collection_name).await?;

        client.delete_collection(collection_name).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_collection_metadata() -> Result<()> {
        let client = get_test_client().await?;
        let collection_name = "test_metadata";

        create_test_collection(&client, collection_name).await?;

        client.collection_metadata(collection_name).await?;

        client.delete_collection(collection_name).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_embedding() -> Result<()> {
        let client = get_test_client().await?;
        let collection_name = "test_insert";

        create_test_collection(&client, collection_name).await?;

        let payload: Payload = json!({"test_field": "test_value"}).try_into().unwrap();
        let point = PointStruct::new(1, vec![0.1; 10], payload);

        client.insert_embedding(collection_name, point).await?;

        client.delete_collection(collection_name).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_embeddings_bulk() -> Result<()> {
        let client = get_test_client().await?;
        let collection_name = "test_bulk_insert";

        create_test_collection(&client, collection_name).await?;

        let mut points = Vec::new();
        for i in 0..5 {
            let payload: Payload = json!({"test_field": format!("value_{}", i)})
                .try_into()
                .unwrap();
            points.push(PointStruct::new(
                i as u64,
                vec![0.1 * i as f32; 10],
                payload,
            ));
        }

        client
            .insert_embeddings_bulk(collection_name, points)
            .await?;

        client.delete_collection(collection_name).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_search_with_vector() -> Result<()> {
        let client = get_test_client().await?;
        let collection_name = "test_search";

        create_test_collection(&client, collection_name).await?;

        let payload: Payload = json!({"test_field": "test_value"}).try_into().unwrap();
        let vector = vec![0.1; 10];
        let point = PointStruct::new(1, vector.clone(), payload);
        client.insert_embedding(collection_name, point).await?;

        let search_result = client
            .search_with_vector(collection_name, vector, Some(1))
            .await?;

        assert_eq!(search_result.result.len(), 1);
        assert_eq!(search_result.result[0].id, Some(PointId::from(1)));

        client.delete_collection(collection_name).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_filtered_search_with_vector() -> Result<()> {
        let client = get_test_client().await?;
        let collection_name = "test_filtered_search";

        create_test_collection(&client, collection_name).await?;

        let payload1: Payload = json!({"category": "A", "value": 10}).try_into().unwrap();
        let payload2: Payload = json!({"category": "B", "value": 20}).try_into().unwrap();

        let point1 = PointStruct::new(1, vec![0.1; 10], payload1);
        let point2 = PointStruct::new(2, vec![0.2; 10], payload2);

        client.insert_embedding(collection_name, point1).await?;
        client.insert_embedding(collection_name, point2).await?;

        let filter = Filter::all([Condition::matches("value", 10)]);

        let search_result = client
            .filtered_search_with_vector(collection_name, vec![0.15; 10], filter, Some(5))
            .await?;

        assert_eq!(search_result.result.len(), 1);
        assert_eq!(search_result.result[0].id, Some(PointId::from(1)));

        client.delete_collection(collection_name).await?;

        Ok(())
    }
}

