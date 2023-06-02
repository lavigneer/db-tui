use std::error::Error;

use cursive::{
    view::Nameable,
    views::{LinearLayout, NamedView},
    CbSink,
};
use mongodb::{bson::doc, Client};

struct CollectionStats {
    storage_size: i32,
    document_count: i32,
    avg_document_size: i32,
    indexes_count: i32,
    total_index_size: i32,
}

struct CollectionStatsView {
    cb_sink: CbSink,
    client: Client,
    view: NamedView<LinearLayout>,
    db: String,
    collection: String,
}

impl CollectionStatsView {
    pub fn new(
        collection: String,
        db: String,
        cb_sink: CbSink,
        client: Client,
    ) -> CollectionStatsView {
        CollectionStatsView {
            cb_sink,
            client,
            view: LinearLayout::vertical().with_name("db_tree"),
            db,
            collection,
        }
    }

    pub async fn load_collection_stats(&mut self) -> Result<CollectionStats, Box<dyn Error>> {
        let stats = self
            .client
            .database(&self.db)
            .run_command(doc! {"collStats": &self.collection }, None)
            .await?;
        let storage_size = stats.get_i32("storageSize").unwrap_or(0);
        let document_count = stats.get_i32("count").unwrap_or(0);
        let avg_document_size = stats.get_i32("avgObjSize").unwrap_or(0);
        let indexes_count = stats.get_i32("nindexes").unwrap_or(0);
        let total_index_size = stats.get_i32("totalIndexSize").unwrap_or(0);
        Ok(CollectionStats {
            storage_size,
            document_count,
            avg_document_size,
            indexes_count,
            total_index_size,
        })
    }
}
