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

pub struct QdrantClientWrapper {
    // TODO: Build a connection pool for QdrantClient
    client: Arc<Qdrant>,
    collection_name: String,
}

impl Clone for QdrantClientWrapper {
    fn clone(&self) -> Self {
        QdrantClientWrapper {
            client: Arc::clone(&self.client),
            collection_name: self.collection_name.clone(),
        }
    }
}

impl QdrantClientWrapper {
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
                        "group_id",
                        FieldType::Keyword,
                    )
                    .field_index_params(KeywordIndexParamsBuilder::default().is_tenant(true)),
                )
                .await?;
        }

        Ok(QdrantClientWrapper {
            client: Arc::new(client),
            collection_name: collection_name.to_string(),
        })
    }

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

    pub async fn filtered_search_with_vector(
        &self,
        collection_name: &str,
        vector: Vec<f32>,
        mut filter: Vec<Condition>,
        limit: Option<u64>,
    ) -> Result<SearchResponse> {
        let limit = limit.unwrap_or(10);

        filter.push(Condition::matches(GROUP_ID, self.collection_name.clone()));

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
