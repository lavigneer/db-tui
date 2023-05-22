use std::{error::Error, fmt::Display};

use cursive::{view::Nameable, views::NamedView, CbSink};
use cursive_tree_view::{Placement, TreeView};
use mongodb::Client;

#[derive(Debug)]
pub enum DbTreeItem {
    DatabaseItem(String),
    CollectionItem(String, String),
}

impl Display for DbTreeItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbTreeItem::DatabaseItem(name) => write!(f, "{}", name),
            DbTreeItem::CollectionItem(_db_name, collection_name) => {
                write!(f, "{}", collection_name)
            }
        }
    }
}

pub struct DbTreeView {
    cb_sink: CbSink,
    client: Client,
    pub tree_view: NamedView<TreeView<DbTreeItem>>,
}

impl DbTreeView {
    pub fn new(cb_sink: CbSink, client: Client) -> DbTreeView {
        DbTreeView {
            cb_sink,
            client,
            tree_view: TreeView::new().with_name("db_tree"),
        }
    }

    pub async fn load_databases(&mut self) -> Result<(), Box<dyn Error>> {
        let databases = self.client.list_databases(None, None).await?;
        let mut tree_view = self.tree_view.get_mut();
        tree_view.clear();

        for database in databases {
            let db_name = database.name;
            let db_row = tree_view.insert_item(
                DbTreeItem::DatabaseItem(db_name.clone()),
                Placement::After,
                0,
            );
            match db_row {
                None => (),
                Some(row) => {
                    let database = self.client.database(&db_name);
                    let collection_names = database.list_collection_names(None).await?;
                    for collection_name in collection_names {
                        tree_view.insert_item(
                            DbTreeItem::CollectionItem(db_name.clone(), collection_name),
                            Placement::LastChild,
                            row,
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
