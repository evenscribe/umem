pub type PointId = qdrant_client::qdrant::PointId;

pub trait QdrantIdentifiable {
    fn get_id(&self) -> impl Into<PointId>;
}
