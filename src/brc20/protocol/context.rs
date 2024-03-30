use std::str::FromStr;

use crate::brc20::datastore::ord::{InscriptionOp, OrdReader, OrdReaderWriter};
use crate::brc20::datastore::{balance, ScriptKey};
use crate::brc20::datastore::{
  Balance, Brc20Reader, Brc20ReaderWriter, Receipt, Tick, TokenInfo, TransferInfo, TransferableLog,
};
use crate::brc20::lru::SimpleLru;
use crate::brc20::protocol::BlockContext;
use crate::index::entry::{InscriptionEntryValue, InscriptionIdValue, OutPointValue, TxidValue};
use crate::inscriptions::InscriptionId;
use crate::SatPoint;
use anyhow::anyhow;
use bitcoin::{Network, OutPoint, Script, TxOut, Txid};
use dal::dal::brc20_token::{Mutation as Brc20TokenWriter, Query as Brc20TokenReader};
use dal::dal::brc20_user_balance::{
  Mutation as Brc20UserBalanceWriter, Query as Brc20UserBalanceReader,
};
use num_traits::cast::ToPrimitive;
use redb::Table;
use sea_orm::{DatabaseConnection, DbErr};

#[allow(non_snake_case)]
pub struct Context<'a, 'db, 'txn> {
  pub(crate) chain: BlockContext,
  pub(crate) tx_out_cache: &'a mut SimpleLru<OutPoint, TxOut>,
  pub(crate) hit: u64,
  pub(crate) miss: u64,
  pub(crate) db: DatabaseConnection,
  pub(crate) network: Network,

  // ord tables
  pub(crate) ORD_TX_TO_OPERATIONS: &'a mut Table<'db, 'txn, &'static TxidValue, &'static [u8]>,
  pub(crate) SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY:
    &'a mut Table<'db, 'txn, u32, InscriptionEntryValue>,
  pub(crate) OUTPOINT_TO_ENTRY: &'a mut Table<'db, 'txn, &'static OutPointValue, &'static [u8]>,

  // BRC20 tables
  pub(crate) BRC20_BALANCES: &'a mut Table<'db, 'txn, &'static str, &'static [u8]>,
  pub(crate) BRC20_TOKEN: &'a mut Table<'db, 'txn, &'static str, &'static [u8]>,
  pub(crate) BRC20_EVENTS: &'a mut Table<'db, 'txn, &'static TxidValue, &'static [u8]>,
  pub(crate) BRC20_TRANSFERABLELOG: &'a mut Table<'db, 'txn, &'static str, &'static [u8]>,
  pub(crate) BRC20_INSCRIBE_TRANSFER: &'a mut Table<'db, 'txn, InscriptionIdValue, &'static [u8]>,
}

impl<'a, 'db, 'txn> OrdReader for Context<'a, 'db, 'txn> {
  type Error = anyhow::Error;

  fn get_inscription_number_by_sequence_number(
    &self,
    sequence_number: u32,
  ) -> crate::Result<i32, Self::Error> {
    get_inscription_number_by_sequence_number(
      self.SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY,
      sequence_number,
    )
    .map_err(|e| anyhow!("failed to get inscription number from state! error: {e}"))?
    .ok_or(anyhow!(
      "failed to get inscription number! error: sequence number {} not found",
      sequence_number
    ))
  }

  fn get_script_key_on_satpoint(
    &mut self,
    satpoint: &SatPoint,
    network: Network,
  ) -> crate::Result<ScriptKey, Self::Error> {
    if let Some(tx_out) = self.tx_out_cache.get(&satpoint.outpoint) {
      self.hit += 1;
      Ok(ScriptKey::from_script(&tx_out.script_pubkey, network))
    } else if let Some(tx_out) = get_txout_by_outpoint(self.OUTPOINT_TO_ENTRY, &satpoint.outpoint)?
    {
      self.miss += 1;
      Ok(ScriptKey::from_script(&tx_out.script_pubkey, network))
    } else {
      Err(anyhow!(
        "failed to get tx out! error: outpoint {} not found",
        &satpoint.outpoint
      ))
    }
  }

  fn get_transaction_operations(
    &self,
    txid: &Txid,
  ) -> crate::Result<Vec<InscriptionOp>, Self::Error> {
    get_transaction_operations(self.ORD_TX_TO_OPERATIONS, txid)
  }
}

impl<'a, 'db, 'txn> OrdReaderWriter for Context<'a, 'db, 'txn> {
  fn save_transaction_operations(
    &mut self,
    txid: &Txid,
    operations: &[InscriptionOp],
  ) -> crate::Result<(), Self::Error> {
    save_transaction_operations(self.ORD_TX_TO_OPERATIONS, txid, operations)
  }
}

impl<'a, 'db, 'txn> Brc20Reader for Context<'a, 'db, 'txn> {
  type Error = anyhow::Error;

  fn get_balances(&self, script_key: &ScriptKey) -> crate::Result<Vec<Balance>, Self::Error> {
    let mut user_balances: Vec<Balance> = vec![];
    let mut err;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let mut page = 0;
      let count_per_page = 100;
      loop {
        let result = Brc20UserBalanceReader::find_in_page(
          &self.db,
          script_key.to_string().as_str(),
          page,
          count_per_page,
        )
        .await;
        match result {
          Ok((balances, page_nums)) => {
            for balance in balances.iter() {
              let temp = Balance {
                tick: Tick::from_str(balance.tick.as_str()).unwrap(),
                overall_balance: balance.overall_balance.to_u128().unwrap(),
                transferable_balance: balance.transferable_balance.to_u128().unwrap(),
              };
              user_balances.push(temp);
            }
            page += 1
          }
          Err(e) => {
            err = e;
            break;
          }
        }
      }
    });

    match err {
      DbErr::RecordNotFound(_) => Ok(user_balances),
      e => Err(anyhow!("failed to get user all balance! error: {e}")),
    }
  }

  fn get_balance(
    &self,
    script_key: &ScriptKey,
    tick: &Tick,
  ) -> crate::Result<Option<Balance>, Self::Error> {
    let mut user_balance: Option<Balance>;
    let mut err;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let result = Brc20UserBalanceReader::find_by_tick(
        &self.db,
        script_key.to_string().as_str(),
        tick.as_str(),
      )
      .await;
      match result {
        Ok(Some(balance)) => {
          user_balance = Some(Balance {
            tick: Tick::from_str(balance.tick.as_str()).unwrap(),
            overall_balance: balance.overall_balance.to_u128().unwrap(),
            transferable_balance: balance.transferable_balance.to_u128().unwrap(),
          });
        }
        Ok(None) => user_balance = None,
        Err(e) => err = e,
      }
    });

    match err {
      DbErr::RecordNotFound(_) => Ok(user_balance),
      e => Err(anyhow!("failed to get user balance! error: {e}")),
    }
  }

  fn get_token_info(&self, tick: &Tick) -> crate::Result<Option<TokenInfo>, Self::Error> {
    let mut token_info: Option<TokenInfo>;
    let mut err;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let result = Brc20TokenReader::find_by_tick(&self.db, tick.as_str()).await;
      match result {
        Ok(Some(token)) => {
          token_info = Some(TokenInfo {
            tick: Tick::from_str(token.tick.as_str()).unwrap(),
            inscription_id: InscriptionId::from_str(&token.inscription_id).unwrap(),
            inscription_number: token.inscription_number,
            supply: token.supply.to_u128().unwrap(),
            minted: token.minted.to_u128().unwrap(),
            limit_per_mint: token.limit_per_mint.to_u128().unwrap(),
            decimal: token.decimal,
            deploy_by: ScriptKey::from_script(
              Script::from_bytes(token.deploy_by.as_bytes()),
              self.network,
            ),
            deployed_number: token.deployed_number,
            deployed_timestamp: token.deployed_timestamp,
            latest_mint_number: token.latest_mint_number,
          });
        }
        Ok(None) => token_info = None,
        Err(e) => err = e,
      }
    });

    match err {
      DbErr::RecordNotFound(_) => Ok(token_info),
      e => Err(anyhow!("failed to get token info! error: {e}")),
    }
  }

  fn get_tokens_info(&self) -> crate::Result<Vec<TokenInfo>, Self::Error> {
    let mut token_infos: Vec<TokenInfo> = vec![];
    let mut err;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let mut page = 0;
      let count_per_page = 100;
      loop {
        let result = Brc20TokenReader::find_in_page(&self.db, page, count_per_page).await;
        match result {
          Ok((tokens, page_nums)) => {
            for token in tokens.iter() {
              let temp = TokenInfo {
                tick: Tick::from_str(token.tick.as_str()).unwrap(),
                inscription_id: InscriptionId::from_str(&token.inscription_id).unwrap(),
                inscription_number: token.inscription_number,
                supply: token.supply.to_u128().unwrap(),
                minted: token.minted.to_u128().unwrap(),
                limit_per_mint: token.limit_per_mint.to_u128().unwrap(),
                decimal: token.decimal,
                deploy_by: ScriptKey::from_script(
                  Script::from_bytes(token.deploy_by.as_bytes()),
                  self.network,
                ),
                deployed_number: token.deployed_number,
                deployed_timestamp: token.deployed_timestamp,
                latest_mint_number: token.latest_mint_number,
              };
              token_infos.push(temp);
            }
            page += 1
          }
          Err(e) => {
            err = e;
            break;
          }
        }
      }
    });

    match err {
      DbErr::RecordNotFound(_) => Ok(token_infos),
      e => Err(anyhow!("failed to get all token info! error: {e}")),
    }
  }

  fn get_transaction_receipts(&self, txid: &Txid) -> crate::Result<Vec<Receipt>, Self::Error> {
    get_transaction_receipts(self.BRC20_EVENTS, txid)
  }

  fn get_transferable(
    &self,
    script: &ScriptKey,
  ) -> crate::Result<Vec<TransferableLog>, Self::Error> {
    get_transferable(self.BRC20_TRANSFERABLELOG, script)
  }

  fn get_transferable_by_tick(
    &self,
    script: &ScriptKey,
    tick: &Tick,
  ) -> crate::Result<Vec<TransferableLog>, Self::Error> {
    get_transferable_by_tick(self.BRC20_TRANSFERABLELOG, script, tick)
  }

  fn get_transferable_by_id(
    &self,
    script: &ScriptKey,
    inscription_id: &InscriptionId,
  ) -> crate::Result<Option<TransferableLog>, Self::Error> {
    get_transferable_by_id(self.BRC20_TRANSFERABLELOG, script, inscription_id)
  }

  fn get_inscribe_transfer_inscription(
    &self,
    inscription_id: &InscriptionId,
  ) -> crate::Result<Option<TransferInfo>, Self::Error> {
    get_inscribe_transfer_inscription(self.BRC20_INSCRIBE_TRANSFER, inscription_id)
  }
}

impl<'a, 'db, 'txn> Brc20ReaderWriter for Context<'a, 'db, 'txn> {
  fn update_token_balance(
    &mut self,
    script_key: &ScriptKey,
    new_balance: Balance,
  ) -> crate::Result<(), Self::Error> {
    update_token_balance(self.BRC20_BALANCES, script_key, new_balance)
  }

  fn insert_token_info(
    &mut self,
    tick: &Tick,
    new_info: &TokenInfo,
  ) -> crate::Result<(), Self::Error> {
    insert_token_info(self.BRC20_TOKEN, tick, new_info)
  }

  fn update_mint_token_info(
    &mut self,
    tick: &Tick,
    minted_amt: u128,
    minted_block_number: u32,
  ) -> crate::Result<(), Self::Error> {
    update_mint_token_info(self.BRC20_TOKEN, tick, minted_amt, minted_block_number)
  }

  fn save_transaction_receipts(
    &mut self,
    txid: &Txid,
    receipt: &[Receipt],
  ) -> crate::Result<(), Self::Error> {
    save_transaction_receipts(self.BRC20_EVENTS, txid, receipt)
  }

  fn insert_transferable(
    &mut self,
    script: &ScriptKey,
    tick: &Tick,
    inscription: &TransferableLog,
  ) -> crate::Result<(), Self::Error> {
    insert_transferable(self.BRC20_TRANSFERABLELOG, script, tick, inscription)
  }

  fn remove_transferable(
    &mut self,
    script: &ScriptKey,
    tick: &Tick,
    inscription_id: &InscriptionId,
  ) -> crate::Result<(), Self::Error> {
    remove_transferable(self.BRC20_TRANSFERABLELOG, script, tick, inscription_id)
  }

  fn insert_inscribe_transfer_inscription(
    &mut self,
    inscription_id: &InscriptionId,
    transfer_info: TransferInfo,
  ) -> crate::Result<(), Self::Error> {
    insert_inscribe_transfer_inscription(
      self.BRC20_INSCRIBE_TRANSFER,
      inscription_id,
      transfer_info,
    )
  }

  fn remove_inscribe_transfer_inscription(
    &mut self,
    inscription_id: &InscriptionId,
  ) -> crate::Result<(), Self::Error> {
    remove_inscribe_transfer_inscription(self.BRC20_INSCRIBE_TRANSFER, inscription_id)
  }
}
