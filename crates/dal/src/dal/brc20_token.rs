use ::entities::brc20_token::{ActiveModel, Column, Entity, Model};
use bigdecimal::FromPrimitive;
use sea_orm::{prelude::Decimal, *};

pub struct Query;

impl Query {
  pub async fn find_by_tick(db: &DbConn, tick: &str) -> Result<Option<Model>, DbErr> {
    Entity::find().filter(Column::Tick.eq(tick)).one(db).await
  }

  pub async fn find_all(db: &DbConn, tick: &str) -> Result<Vec<Model>, DbErr> {
    Entity::find().filter(Column::Tick.eq(tick)).all(db).await
  }

  // If ok, returns (scanner height models, num pages).
  pub async fn find_in_page(
    db: &DbConn,
    page: u64,
    count_per_page: u64,
  ) -> Result<(Vec<Model>, u64), DbErr> {
    // Setup paginator
    let paginator = Entity::find()
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
    let data = form_data.clone().into_active_model();
    Entity::insert(data).exec(db).await
  }

  pub async fn update_mint_info<C>(
    db: &C,
    tick: &str,
    minted_amt: u128,
    minted_block_number: u32,
  ) -> Result<Model, DbErr>
  where
    C: ConnectionTrait,
  {
    let token: ActiveModel = Entity::find()
      .filter(Column::Tick.eq(tick))
      .one(db)
      .await?
      .ok_or(DbErr::Custom("Cannot find balance.".to_owned()))
      .map(Into::into)?;

    ActiveModel {
      id: token.id,
      minted: Set(Decimal::from_u128(minted_amt).unwrap()),
      latest_mint_number: Set(minted_block_number),
      ..Default::default()
    }
    .update(db)
    .await
  }
}
