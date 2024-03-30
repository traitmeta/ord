use ::entities::brc20_transferable_log::{ActiveModel, Column, Entity, Model};
use sea_orm::*;

pub struct Query;

impl Query {
  pub async fn get_transferable_by_id(
    db: &DbConn,
    cript_hash: &str,
    inscription_id: &str,
  ) -> Result<Option<Model>, DbErr> {
    Entity::find()
      .filter(Column::Owner.eq(cript_hash))
      .filter(Column::InscriptionId.eq(inscription_id))
      .one(db)
      .await
  }

  pub async fn get_transferable_by_tick(
    db: &DbConn,
    script_hash: &str,
    tick: &str,
  ) -> Result<Vec<Model>, DbErr> {
    Entity::find()
      .filter(Column::Tick.eq(tick))
      .filter(Column::Owner.eq(script_hash))
      .all(db)
      .await
  }

  // If ok, returns (scanner height models, num pages).
  pub async fn find_in_page(
    db: &DbConn,
    script_hash: &str,
    page: u64,
    count_per_page: u64,
  ) -> Result<(Vec<Model>, u64), DbErr> {
    // Setup paginator
    let paginator = Entity::find()
      .filter(Column::Owner.eq(script_hash))
      .order_by_asc(Column::Id)
      .paginate(db, count_per_page);
    let num_pages = paginator.num_pages().await?;

    // Fetch paginated posts
    paginator.fetch_page(page - 1).await.map(|p| (p, num_pages))
  }
}

pub struct Mutation;

impl Mutation {
  pub async fn creates<C>(db: &C, form_datas: &[Model]) -> Result<InsertResult<ActiveModel>, DbErr>
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

  pub async fn create<C>(db: &C, form_data: &Model) -> Result<InsertResult<ActiveModel>, DbErr>
  where
    C: ConnectionTrait,
  {
    let token = form_data.clone().into_active_model();

    Entity::insert(token).exec(db).await
  }
}
