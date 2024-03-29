use ::entities::brc20_user_balance::{ActiveModel, Column, Entity, Model};
use sea_orm::*;

pub struct Query;

impl Query {
  pub async fn find_by_tick(db: &DbConn, user: &str, tick: &str) -> Result<Option<Model>, DbErr> {
    Entity::find()
      .filter(Column::Tick.eq(tick))
      .filter(Column::SctiptHash.eq(user))
      .one(db)
      .await
  }

  // If ok, returns (scanner height models, num pages).
  pub async fn find_in_page(
    db: &DbConn,
    user: &str,
    page: u64,
    count_per_page: u64,
  ) -> Result<(Vec<Model>, u64), DbErr> {
    // Setup paginator
    let paginator = Entity::find()
      .filter(Column::SctiptHash.eq(user))
      .order_by_asc(Column::Id)
      .paginate(db, count_per_page);
    let num_pages = paginator.num_pages().await?;

    // Fetch paginated posts
    paginator.fetch_page(page - 1).await.map(|p| (p, num_pages))
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

  pub async fn update_balance<C>(db: &C, form_data: &Model) -> Result<Model, DbErr>
  where
    C: ConnectionTrait,
  {
    let mut token = form_data.clone().into_active_model();
    token.overall_balance = Set(form_data.overall_balance.to_owned());
    token.transferable_balance = Set(form_data.transferable_balance);

    Entity::update(token)
      .filter(Column::SctiptHash.eq(form_data.sctipt_hash.clone()))
      .exec(db)
      .await
  }
}
