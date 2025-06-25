use anyhow::Result;
use qdrant_client::{
    Payload, Qdrant,
    qdrant::{
        Condition, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder, Distance, FieldType,
        Filter, HnswConfigDiffBuilder, KeywordIndexParamsBuilder, PointId, PointStruct,
        QuantizationType, ScalarQuantizationBuilder, SearchPointsBuilder, SearchResponse,
        UpsertPointsBuilder, VectorParamsBuilder,
    },
};
use std::sync::Arc;
use uuid::Uuid;

const GROUP_ID: &str = "group_id";

pub struct MemoryVectorStore {
    // TODO: Build a connection pool for QdrantClient
    client: Arc<Qdrant>,
    collection_name: String,
}

impl Clone for QdrantClientWrapper {
    /// Creates a new `QdrantClientWrapper` instance with cloned client and collection name.
    ///
    /// The cloned wrapper shares the same underlying Qdrant client via reference counting,
    /// while maintaining an independent copy of the collection name.
    fn clone(&self) -> Self {
        MemoryVectorStore {
            client: Arc::clone(&self.client),
            collection_name: self.collection_name.clone(),
        }
    }
}

impl QdrantClientWrapper {
    /// Asynchronously creates a new `QdrantClientWrapper` for the specified collection.
    ///
    /// If the collection does not exist, it is created with a 1024-dimensional vector configuration (cosine distance), HNSW index, 8-bit integer quantization, and a tenant-aware keyword index on the `"group_id"` field.
    ///
    /// # Arguments
    ///
    /// * `url` - The Qdrant server URL.
    /// * `api_key` - The API key for authentication.
    /// * `collection_name` - The name of the collection to use or create.
    ///
    /// # Returns
    ///
    /// A `QdrantClientWrapper` instance configured for the specified collection.
    ///
    /// # Examples
    ///
    /// ```
    /// let wrapper = QdrantClientWrapper::new("http://localhost:6333", "my-api-key", "my_collection").await?;
    /// ```
    pub async fn new(url: &str, api_key: &str, collection_name: &str) -> Result<Self> {
        let client = Qdrant::from_url(url).api_key(api_key).build()?;

        if !client.collection_exists(collection_name).await? {
            client
                .create_collection(
                    CreateCollectionBuilder::new(collection_name)
                        .vectors_config(VectorParamsBuilder::new(1024, Distance::Cosine))
                        .hnsw_config(HnswConfigDiffBuilder::default().payload_m(16).m(0))
                        .quantization_config(
                            ScalarQuantizationBuilder::default()
                                .r#type(QuantizationType::Int8.into())
                                .always_ram(true),
                        ),
                )
                .await?;

            client
                .create_field_index(
                    CreateFieldIndexCollectionBuilder::new(
                        collection_name,
                        GROUP_ID,
                        FieldType::Keyword,
                    )
                    .field_index_params(KeywordIndexParamsBuilder::default().is_tenant(true)),
                )
                .await?;
        }

        Ok(MemoryVectorStore {
            client: Arc::new(client),
            collection_name: collection_name.to_string(),
        })
    }

    /// Inserts a single vector embedding with an associated payload and user ID into the collection.
    ///
    /// The user ID is added to the payload under the `"group_id"` key to ensure tenant isolation. A new UUID is generated as the point ID for the inserted vector.
    ///
    /// # Arguments
    ///
    /// * `payload` - The payload data to associate with the vector.
    /// * `vectors` - The vector embedding to insert.
    /// * `user_id` - The user identifier to associate with the point.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the insertion succeeds; otherwise, returns an error.
    pub async fn insert_embedding(
        &self,
        mut payload: Payload,
        vectors: Vec<f32>,
        user_id: &str,
    ) -> Result<()> {
        payload.insert(GROUP_ID.to_string(), user_id.to_string());

        self.client
            .upsert_points(UpsertPointsBuilder::new(
                self.collection_name.clone(),
                [PointStruct::new(
                    PointId::from(Uuid::new_v4().to_string()),
                    vectors,
                    payload,
                )],
            ))
            .await?;

        Ok(())
    }

    /**
     * (Payload, Embeddings, user_id)
     */
    /// Inserts multiple vector embeddings with associated payloads and user IDs in bulk.
    ///
    /// Each tuple in the input vector contains a payload, a vector, and a user ID. The user ID is added to the payload under the `"group_id"` key, and each point is assigned a new UUID as its ID. All points are upserted into the collection in a single batch operation.
    ///
    /// # Examples
    ///
    /// ```
    /// let points = vec![
    ///     (payload1, vec![0.1, 0.2, 0.3], "user1"),
    ///     (payload2, vec![0.4, 0.5, 0.6], "user2"),
    /// ];
    /// wrapper.insert_embeddings_bulk(points).await?;
    /// ```
    pub async fn insert_embeddings_bulk<I>(
        &self,
        mut points: Vec<(Payload, Vec<f32>, &str)>,
    ) -> Result<()> {
        let ps = points
            .iter_mut()
            .map(|(payload, vectors, user_id)| {
                payload.insert(GROUP_ID.to_string(), user_id.to_string());
                PointStruct::new(
                    PointId::from(Uuid::new_v4().to_string()),
                    std::mem::take(vectors),
                    std::mem::take(payload),
                )
            })
            .collect::<Vec<_>>();

        self.client
            .upsert_points(UpsertPointsBuilder::new(self.collection_name.clone(), ps))
            .await?;
        Ok(())
    }

    /// Performs a vector similarity search within the stored collection, returning results filtered by user ID.
    ///
    /// Searches for points most similar to the provided vector, limiting results to those where the `"group_id"` payload matches the given user ID. The number of results returned can be controlled with the `limit` parameter (default is 10).
    ///
    /// # Parameters
    /// - `vector`: The query vector to search against.
    /// - `limit`: Optional maximum number of results to return (defaults to 10 if not specified).
    /// - `user_id`: The user identifier used to filter search results by `"group_id"`.
    ///
    /// # Returns
    /// A `SearchResponse` containing the matching points and their payloads.
    ///
    /// # Examples
    ///
    /// ```
    /// let response = wrapper
    ///     .search_with_vector(query_vector, Some(5), "user123")
    ///     .await?;
    /// assert!(response.result.len() <= 5);
    /// ```
    pub async fn search_with_vector(
        &self,
        vector: Vec<f32>,
        limit: Option<u64>,
        user_id: &str,
    ) -> Result<SearchResponse> {
        let limit = limit.unwrap_or(10);
        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(self.collection_name.clone(), vector, limit)
                    .with_payload(true)
                    .filter(Filter::must([Condition::matches(
                        GROUP_ID,
                        user_id.to_string(),
                    )])),
            )
            .await?;

        Ok(search_result)
    }

    /// Performs a filtered vector similarity search on a specified collection, including tenant isolation.
    ///
    /// Adds a filter condition to restrict results to points where the `"group_id"` matches the wrapper's collection name. Combines this with any additional provided filter conditions. Returns search results with payloads included.
    ///
    /// # Parameters
    /// - `collection_name`: The name of the collection to search.
    /// - `vector`: The query vector for similarity search.
    /// - `filter`: Additional filter conditions to apply (combined with tenant isolation).
    /// - `limit`: Optional maximum number of results to return (defaults to 10).
    ///
    /// # Returns
    /// The search response containing matching points and their payloads.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut conditions = vec![Condition::matches("status", "active")];
    /// let response = wrapper
    ///     .filtered_search_with_vector("my_collection", query_vec, conditions, Some(5))
    ///     .await?;
    /// assert!(response.result.len() <= 5);
    /// ```
    pub async fn filtered_search_with_vector(
        &self,
        collection_name: &str,
        vector: Vec<f32>,
        mut filter: Vec<Condition>,
        limit: Option<u64>,
        user_id: &str,
    ) -> Result<SearchResponse> {
        let limit = limit.unwrap_or(10);

        filter.push(Condition::matches(GROUP_ID, user_id.to_string()));

        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(collection_name, vector, limit)
                    .with_payload(true)
                    .filter(Filter::all(filter)),
            )
            .await?;

        Ok(search_result)
    }
}
