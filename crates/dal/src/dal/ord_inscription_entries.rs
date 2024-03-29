use ::entities::ord_inscription_entries::{ActiveModel, Column, Entity, Model};
use sea_orm::*;

pub struct Query;

impl Query {
  pub async fn finder_by_sequence_number(
    db: &DbConn,
    seq_number: u32,
  ) -> Result<Option<Model>, DbErr> {
    Entity::find()
      .filter(Column::SequenceNumber.eq(seq_number))
      .limit(1)
      .one(db)
      .await
  }

  pub async fn finder_by_inscription_number(
    db: &DbConn,
    inscription_num: u32,
  ) -> Result<Option<Model>, DbErr> {
    Entity::find()
      .filter(Column::InscriptionNumber.eq(inscription_num))
      .limit(1)
      .one(db)
      .await
  }

  pub async fn find_by_id(db: &DbConn, outpoint: Vec<u8>) -> Result<Vec<Model>, DbErr> {
    Entity::find().filter(Column::Id.eq(outpoint)).all(db).await
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

  pub async fn create_one<C>(db: &C, form_data: &Model) -> Result<Model, DbErr>
  where
    C: ConnectionTrait,
  {
    let data = form_data.clone().into_active_model();
    data.insert(db).await
  }
}
