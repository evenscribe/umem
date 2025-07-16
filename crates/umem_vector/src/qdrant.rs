use anyhow::Result;
use chrono;
use qdrant_client::{
    Payload, Qdrant,
    qdrant::{
        Condition, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder, DeletePointsBuilder,
        Distance, FieldType, Filter, HnswConfigDiffBuilder, KeywordIndexParamsBuilder, PointId,
        PointStruct, PointVectors, PointsIdsList, QuantizationType, ScalarQuantizationBuilder,
        ScrollPointsBuilder, ScrollResponse, SearchPointsBuilder, SearchResponse,
        SetPayloadPointsBuilder, UpdatePointVectorsBuilder, UpsertPointsBuilder,
        VectorParamsBuilder,
    },
};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

pub struct QdrantVectorStore {
    client: Qdrant,
    collection_name: String,
}

impl QdrantVectorStore {
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
                        "user_id",
                        FieldType::Keyword,
                    )
                    .field_index_params(KeywordIndexParamsBuilder::default().is_tenant(true)),
                )
                .await?;
        }

        Ok(QdrantVectorStore {
            client: client,
            collection_name: collection_name.to_string(),
        })
    }

    pub async fn insert_embedding<S: Serialize>(
        &self,
        payload: S,
        vectors: Vec<f32>,
    ) -> Result<()> {
        let mut payload = Payload::try_from(json!(payload))?;
        let uuid = Uuid::new_v4().to_string();
        payload.insert("memory_id", uuid.as_str());
        let now = chrono::prelude::Utc::now().timestamp();
        payload.insert("created_at", now);
        payload.insert("updated_at", now);
        self.client
            .upsert_points(UpsertPointsBuilder::new(
                self.collection_name.as_str(),
                [PointStruct::new(
                    PointId::from(uuid.as_str()),
                    vectors,
                    payload,
                )],
            ))
            .await?;
        Ok(())
    }

    pub async fn insert_embeddings_bulk<S: Serialize>(
        &self,
        points: Vec<(S, Vec<f32>)>,
    ) -> Result<()> {
        self.client
            .upsert_points(UpsertPointsBuilder::new(
                self.collection_name.as_str(),
                points
                    .into_iter()
                    .map(|(payload, vectors)| {
                        let mut payload =
                            Payload::try_from(json!(payload)).expect("Couldn't parse to payload.");
                        let uuid = Uuid::new_v4().to_string();
                        payload.insert("memory_id", uuid.as_str());
                        let now = chrono::prelude::Utc::now().timestamp();
                        payload.insert("created_at", now);
                        payload.insert("updated_at", now);
                        PointStruct::new(PointId::from(uuid.as_str()), vectors, payload)
                    })
                    .collect::<Vec<_>>(),
            ))
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
                SearchPointsBuilder::new(self.collection_name.as_str(), vector, limit)
                    .with_payload(true)
                    .filter(Filter::must([Condition::matches(
                        "user_id",
                        user_id.to_string(),
                    )])),
            )
            .await?;

        Ok(search_result)
    }

    pub async fn search_with_payload(
        &self,
        payload: Vec<(String, String)>,
        limit: Option<u32>,
    ) -> Result<ScrollResponse> {
        let search_result = self
            .client
            .scroll(
                ScrollPointsBuilder::new(self.collection_name.as_str())
                    .filter(Filter::must(
                        payload
                            .into_iter()
                            .map(|(field, value)| Condition::matches(field, value)),
                    ))
                    .limit(limit.unwrap_or(10))
                    .with_payload(true)
                    .with_vectors(false),
            )
            .await?;

        Ok(search_result)
    }

    pub async fn delete_point(&self, id: &str) -> Result<()> {
        self.client
            .delete_points(
                DeletePointsBuilder::new(self.collection_name.as_str())
                    .points(PointsIdsList {
                        ids: vec![id.into()],
                    })
                    .wait(true),
            )
            .await?;

        Ok(())
    }

    pub async fn delete_points_bulk(&self, ids: Vec<&str>) -> Result<()> {
        self.client
            .delete_points(
                DeletePointsBuilder::new(self.collection_name.as_str())
                    .points(PointsIdsList {
                        ids: ids.into_iter().map(|id| id.into()).collect(),
                    })
                    .wait(true),
            )
            .await?;

        Ok(())
    }

    pub async fn update_point<S: Serialize>(
        &self,
        id: &str,
        vectors: Option<Vec<f32>>,
        payload: Option<S>,
    ) -> Result<()> {
        self.client
            .update_vectors(
                UpdatePointVectorsBuilder::new(
                    self.collection_name.as_str(),
                    vec![PointVectors {
                        id: Some(id.into()),
                        vectors: vectors.map(|v| v.into()),
                    }],
                )
                .wait(true),
            )
            .await?;

        if let Some(payload) = payload {
            let mut payload = Payload::try_from(json!(payload))?;
            let now = chrono::prelude::Utc::now().timestamp();
            payload.insert("updated_at", now);
            self.client
                .set_payload(
                    SetPayloadPointsBuilder::new(self.collection_name.as_str(), payload)
                        .points_selector(PointsIdsList {
                            ids: vec![id.into()],
                        })
                        .wait(true),
                )
                .await?;
        }

        Ok(())
    }
}
