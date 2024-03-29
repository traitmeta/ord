

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "brc20_tx_receipt")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: i64,

  pub inscription_id: String,
  pub inscription_number: i32,
  #[sea_orm(column_type = "Decimal(Some((20, 0)))", nullable)]
  pub amount: Decimal,
  pub tick: String,
  pub owner: String,
}


#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
