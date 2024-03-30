use ::entities::brc20_tx_receipt::{ActiveModel, Column, Entity, Model};
use sea_orm::*;

pub struct Query;

impl Query {
  pub async fn get_transaction_receipts(db: &DbConn, tx_id: &str) -> Result<Vec<Model>, DbErr> {
    Entity::find().filter(Column::TxId.eq(tx_id)).all(db).await
  }

  // TODO use select 
  pub async fn get_inscribe_transfer_inscription(db: &DbConn, inscription_id: &str) -> Result<Option<Model>, DbErr> {
    Entity::find().filter(Column::InscriptionId.eq(inscription_id)).one(db).await
  }
}

pub struct Mutation;

impl Mutation {
  pub async fn create<C>(db: &C, form_datas: &[Model]) -> Result<InsertResult<ActiveModel>, DbErr>
  where
    C: ConnectionTrait,
  {
    let mut batch = vec![];
    for form_data in form_datas.iter() {
      let data = form_data.clone().into_active_model();
      batch.push(data);
    }

    Entity::insert_many(batch).exec(db).await
  }
}
