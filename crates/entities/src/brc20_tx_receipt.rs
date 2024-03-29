use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// Define the `Category` active enum
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum OperationType {
  #[sea_orm(num_value = 0)]
  Deploy,
  #[sea_orm(num_value = 1)]
  Mint,
  #[sea_orm(num_value = 2)]
  InscribeTransfer,
  #[sea_orm(num_value = 3)]
  Transfer,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "brc20_tx_receipt")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: i64,

  pub inscription_id: String,
  pub inscription_number: i32,
  pub old_satpoint: String,
  pub new_satpoint: String,
  pub op: OperationType,
  pub from: String,
  pub to: String,
  #[sea_orm(column_type = "Decimal(Some((20, 0)))", nullable)]
  pub supply: Option<Decimal>,
  #[sea_orm(column_type = "Decimal(Some((20, 0)))", nullable)]
  pub limit_per_mint: Option<Decimal>,
  pub decimal: Option<u8>,
  pub tick: String,
  #[sea_orm(column_type = "Decimal(Some((20, 0)))", nullable)]
  pub amount: Option<Decimal>,
  pub msg: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
