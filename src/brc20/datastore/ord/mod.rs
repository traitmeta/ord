pub use self::operation::{Action, InscriptionOp};
use bitcoin::Network;

use crate::brc20::datastore::ScriptKey;
use crate::SatPoint;
use {
  crate::Result,
  bitcoin::Txid,
  std::fmt::{Debug, Display},
};

pub mod operation;
pub mod redb;

pub trait OrdReader {
  type Error: Debug + Display;
  fn get_inscription_number_by_sequence_number(
    &self,
    sequence_number: u32,
  ) -> Result<i32, Self::Error>;

  fn get_script_key_on_satpoint(
    &mut self,
    satpoint: &SatPoint,
    network: Network,
  ) -> Result<ScriptKey, Self::Error>;

  fn get_transaction_operations(&self, txid: &Txid) -> Result<Vec<InscriptionOp>, Self::Error>;
}

pub trait OrdReaderWriter: OrdReader {
  fn save_transaction_operations(
    &mut self,
    txid: &Txid,
    operations: &[InscriptionOp],
  ) -> Result<(), Self::Error>;
}
