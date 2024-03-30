use std::str::FromStr;

use crate::brc20::datastore::errors::BRC20Error;
use crate::brc20::datastore::ord::redb::table::get_txout_by_outpoint;
use crate::brc20::datastore::ord::{InscriptionOp, OrdReader, OrdReaderWriter};
use crate::brc20::datastore::{
  Balance, Brc20Reader, Brc20ReaderWriter, InscripbeTransferEvent, MintEvent, Receipt, Tick,
  TokenInfo, TransferEvent, TransferInfo, TransferableLog,
};
use crate::brc20::datastore::{DeployEvent, Event, Event::Deploy, ScriptKey};
use crate::brc20::lru::SimpleLru;
use crate::brc20::protocol::BlockContext;
use crate::index::entry::{InscriptionEntryValue, InscriptionIdValue, OutPointValue, TxidValue};
use crate::inscriptions::InscriptionId;
use crate::SatPoint;
use anyhow::anyhow;
use bitcoin::{Network, OutPoint, Script, TxOut, Txid};
use dal::dal::brc20_token::{Mutation as Brc20TokenWriter, Query as Brc20TokenReader};
use dal::dal::brc20_trasnferable_log::{
  Mutation as Brc20TxableLogWriter, Query as Brc20TxableLogReader,
};
use dal::dal::brc20_tx_receipt::{Mutation as Brc20TxReceiptWriter, Query as Brc20TxReceiptReader};
use dal::dal::brc20_user_balance::{
  Mutation as Brc20UserBalanceWriter, Query as Brc20UserBalanceReader,
};
use entities::brc20_token::Model as Brc20TokenModel;
use entities::brc20_transferable_log::Model as Brc20TxableLog;
use entities::brc20_tx_receipt::Model as Brc20TxReceipt;
use entities::brc20_user_balance::Model as Brc20UserBalance;

use num_traits::cast::ToPrimitive;
use num_traits::FromPrimitive;
use redb::Table;
use sea_orm::prelude::Decimal;
use sea_orm::{DatabaseConnection, DbErr};

#[allow(non_snake_case)]
pub struct Context<'a, 'db, 'txn> {
  pub(crate) chain: BlockContext,
  pub(crate) tx_out_cache: &'a mut SimpleLru<OutPoint, TxOut>,
  pub(crate) hit: u64,
  pub(crate) miss: u64,
  pub(crate) db: DatabaseConnection,

  // ord tables
  pub(crate) ORD_TX_TO_OPERATIONS: &'a mut Table<'db, 'txn, &'static TxidValue, &'static [u8]>,
  pub(crate) SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY:
    &'a mut Table<'db, 'txn, u32, InscriptionEntryValue>,
  pub(crate) OUTPOINT_TO_ENTRY: &'a mut Table<'db, 'txn, &'static OutPointValue, &'static [u8]>,
}

impl<'a, 'db, 'txn> OrdReader for Context<'a, 'db, 'txn> {
  type Error = anyhow::Error;

  fn get_inscription_number_by_sequence_number(
    &self,
    sequence_number: u32,
  ) -> crate::Result<i32, Self::Error> {
    Ok(0)

    // get_inscription_number_by_sequence_number(
    //   self.SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY,
    //   sequence_number,
    // )
    // .map_err(|e| anyhow!("failed to get inscription number from state! error: {e}"))?
    // .ok_or(anyhow!(
    //   "failed to get inscription number! error: sequence number {} not found",
    //   sequence_number
    // ))
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
    Ok(vec![])

    // get_transaction_operations(self.ORD_TX_TO_OPERATIONS, txid)
  }
}

impl<'a, 'db, 'txn> OrdReaderWriter for Context<'a, 'db, 'txn> {
  fn save_transaction_operations(
    &mut self,
    txid: &Txid,
    operations: &[InscriptionOp],
  ) -> crate::Result<(), Self::Error> {
    Ok(())
    // save_transaction_operations(self.ORD_TX_TO_OPERATIONS, txid, operations)
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
              self.chain.network,
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
                  self.chain.network,
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
    let mut tx_receipts: Vec<Receipt> = vec![];
    let mut err: Option<BRC20Error>;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let mut page = 0;
      let count_per_page = 100;
      loop {
        let result =
          Brc20TxReceiptReader::get_transaction_receipts(&self.db, txid.to_string().as_str()).await;
        match result {
          Ok(receipts) => {
            for receipt in receipts.iter() {
              let mut event: Result<Event, BRC20Error>;
              match receipt.op {
                entities::brc20_tx_receipt::OperationType::Deploy => {
                  let supply = match receipt.supply {
                    Some(supply) => supply.to_u128().unwrap(),
                    None => {
                      err = Some(BRC20Error::InvalidSupply("not found in db".to_string()));
                      return;
                    }
                  };
                  let limit_per_mint = match receipt.limit_per_mint {
                    Some(limit_per_mint) => limit_per_mint.to_u128().unwrap(),
                    None => {
                      err = Some(BRC20Error::InvalidInteger(
                        "limit per mint not found in db".to_string(),
                      ));
                      return;
                    }
                  };
                  event = Ok(Event::Deploy(DeployEvent {
                    supply,
                    limit_per_mint,
                    decimal: receipt.decimal.map_or(18, |d| d),
                    tick: Tick::from_str(receipt.tick.as_str()).unwrap(),
                  }))
                }
                entities::brc20_tx_receipt::OperationType::Mint => {
                  let amount = match receipt.amount {
                    Some(amount) => amount.to_u128().unwrap(),
                    None => {
                      err = Some(BRC20Error::InvalidInteger(
                        "not found amount in db".to_string(),
                      ));
                      return;
                    }
                  };

                  event = Ok(Event::Mint(MintEvent {
                    tick: Tick::from_str(receipt.tick.as_str()).unwrap(),
                    amount,
                    msg: receipt.msg,
                  }))
                }
                entities::brc20_tx_receipt::OperationType::InscribeTransfer => {
                  let amount = match receipt.amount {
                    Some(amount) => amount.to_u128().unwrap(),
                    None => {
                      err = Some(BRC20Error::InvalidInteger(
                        "not found amount in db".to_string(),
                      ));
                      return;
                    }
                  };

                  event = Ok(Event::InscribeTransfer(InscripbeTransferEvent {
                    tick: Tick::from_str(receipt.tick.as_str()).unwrap(),
                    amount,
                    msg: receipt.msg,
                  }))
                }
                entities::brc20_tx_receipt::OperationType::Transfer => {
                  let amount = match receipt.amount {
                    Some(amount) => amount.to_u128().unwrap(),
                    None => {
                      err = Some(BRC20Error::InvalidInteger(
                        "not found amount in db".to_string(),
                      ));
                      return;
                    }
                  };

                  event = Ok(Event::Transfer(TransferEvent {
                    tick: Tick::from_str(receipt.tick.as_str()).unwrap(),
                    amount,
                    msg: receipt.msg,
                  }))
                }
              }
              let temp = Receipt {
                inscription_id: InscriptionId::from_str(&receipt.inscription_id).unwrap(),
                inscription_number: receipt.inscription_number,
                old_satpoint: SatPoint::from_str(receipt.old_satpoint.as_str()).unwrap(),
                new_satpoint: SatPoint::from_str(receipt.new_satpoint.as_str()).unwrap(),
                op: receipt.op,
                from: ScriptKey::from_script(
                  Script::from_bytes(receipt.from.as_bytes()),
                  self.chain.network,
                ),
                to: ScriptKey::from_script(
                  Script::from_bytes(receipt.to.as_bytes()),
                  self.chain.network,
                ),
                result: event,
              };

              tx_receipts.push(temp);
            }
            page += 1
          }
          Err(e) => {
            err = Some(BRC20Error::TxNotFound(txid.to_string()));
            return;
          }
        }
      }
    });

    match err {
      None => Ok(tx_receipts),
      Some(e) => Err(anyhow!("failed to get all token info! error: {e}")),
    }
  }

  fn get_transferable(
    &self,
    script: &ScriptKey,
  ) -> crate::Result<Vec<TransferableLog>, Self::Error> {
    let mut transferable_logs: Vec<TransferableLog> = vec![];
    let mut err;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let mut page = 0;
      let count_per_page = 100;
      loop {
        let result = Brc20TxableLogReader::find_in_page(
          &self.db,
          script.to_string().as_str(),
          page,
          count_per_page,
        )
        .await;
        match result {
          Ok((logs, page_nums)) => {
            for log in logs.iter() {
              let temp = TransferableLog {
                tick: Tick::from_str(log.tick.as_str()).unwrap(),
                inscription_id: InscriptionId::from_str(&log.inscription_id).unwrap(),
                inscription_number: log.inscription_number,
                amount: log.amount.to_u128().unwrap(),
                owner: ScriptKey::from_script(
                  Script::from_bytes(log.owner.as_bytes()),
                  self.chain.network,
                ),
              };
              transferable_logs.push(temp);
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
      DbErr::RecordNotFound(_) => Ok(transferable_logs),
      e => Err(anyhow!("failed to get all token info! error: {e}")),
    }
  }

  fn get_transferable_by_tick(
    &self,
    script: &ScriptKey,
    tick: &Tick,
  ) -> crate::Result<Vec<TransferableLog>, Self::Error> {
    let mut transferable_logs: Vec<TransferableLog> = vec![];
    let mut err;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let result = Brc20TxableLogReader::get_transferable_by_tick(
        &self.db,
        script.to_string().as_str(),
        tick.as_str(),
      )
      .await;
      match result {
        Ok(logs) => {
          for log in logs.iter() {
            let temp = TransferableLog {
              tick: Tick::from_str(log.tick.as_str()).unwrap(),
              inscription_id: InscriptionId::from_str(&log.inscription_id).unwrap(),
              inscription_number: log.inscription_number,
              amount: log.amount.to_u128().unwrap(),
              owner: ScriptKey::from_script(
                Script::from_bytes(log.owner.as_bytes()),
                self.chain.network,
              ),
            };
            transferable_logs.push(temp);
          }
        }
        Err(e) => {
          err = e;
        }
      }
    });

    match err {
      DbErr::RecordNotFound(_) => Ok(transferable_logs),
      e => Err(anyhow!("failed to get all token info! error: {e}")),
    }
  }

  fn get_transferable_by_id(
    &self,
    script: &ScriptKey,
    inscription_id: &InscriptionId,
  ) -> crate::Result<Option<TransferableLog>, Self::Error> {
    let mut transferable_log: Option<TransferableLog>;
    let mut err: Option<DbErr>;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let result = Brc20TxableLogReader::get_transferable_by_id(
        &self.db,
        script.to_string().as_str(),
        inscription_id.to_string().as_str(),
      )
      .await;
      match result {
        Ok(Some(log)) => {
          transferable_log = Some(TransferableLog {
            tick: Tick::from_str(log.tick.as_str()).unwrap(),
            inscription_id: InscriptionId::from_str(&log.inscription_id).unwrap(),
            inscription_number: log.inscription_number,
            amount: log.amount.to_u128().unwrap(),
            owner: ScriptKey::from_script(
              Script::from_bytes(log.owner.as_bytes()),
              self.chain.network,
            ),
          });
        }
        Ok(None) => transferable_log = None,
        Err(e) => err = Some(e),
      }
    });

    match err {
      Some(e) => Err(anyhow!("get transferable log by id fail! {e}")),
      None => Ok(transferable_log),
    }
  }

  fn get_inscribe_transfer_inscription(
    &self,
    inscription_id: &InscriptionId,
  ) -> crate::Result<Option<TransferInfo>, Self::Error> {
    let mut transferable_logs: Option<TransferInfo>;
    let mut err;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let result = Brc20TxReceiptReader::get_inscribe_transfer_inscription(
        &self.db,
        inscription_id.to_string().as_str(),
      )
      .await;
      match result {
        Ok(Some(receipt)) => {
          let amount = match receipt.amount {
            Some(amount) => amount.to_u128().unwrap(),
            None => {
              err = Some(DbErr::AttrNotSet("amount".to_string()));
              return;
            }
          };
          transferable_logs = Some(TransferInfo {
            tick: Tick::from_str(receipt.tick.as_str()).unwrap(),
            amt: amount,
          });
        }
        Ok(None) => transferable_logs = None,
        Err(e) => {
          err = Some(e);
        }
      }
    });

    match err {
      None => Ok(transferable_logs),
      Some(e) => Err(anyhow!("failed to get all token info! error: {e}")),
    }
  }
}

impl<'a, 'db, 'txn> Brc20ReaderWriter for Context<'a, 'db, 'txn> {
  fn update_token_balance(
    &mut self,
    script_key: &ScriptKey,
    new_balance: Balance,
  ) -> crate::Result<(), Self::Error> {
    let mut user_balance = Brc20UserBalance {
      tick: new_balance.tick.to_string(),
      overall_balance: Decimal::from_u128(new_balance.overall_balance).unwrap(),
      transferable_balance: Decimal::from_u128(new_balance.transferable_balance).unwrap(),
      id: 0,
      address: None,
      sctipt_hash: None,
    };

    let mut addr: String;
    match script_key {
      ScriptKey::Address(a) => addr = a.clone().assume_checked().to_string(),
      ScriptKey::ScriptHash(script) => addr = script_key.to_string(),
    }

    let mut err: Option<DbErr>;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let result = Brc20UserBalanceWriter::update_balance_by_tick(
        &self.db,
        addr.as_str(),
        new_balance.tick.to_string().as_str(),
        &user_balance,
      )
      .await;
      match result {
        Ok(_) => err = None,
        Err(e) => err = Some(e),
      }
    });

    match err {
      None => Ok(()),
      Some(e) => Err(anyhow!("failed to get all token info! error: {e}")),
    }
  }

  fn insert_token_info(
    &mut self,
    tick: &Tick,
    new_info: &TokenInfo,
  ) -> crate::Result<(), Self::Error> {
    let mut token = Brc20TokenModel {
      id: 0,
      tick: new_info.tick.to_string(),
      inscription_id: new_info.inscription_id.to_string(),
      inscription_number: new_info.inscription_number,
      supply: Decimal::from_u128(new_info.supply).unwrap(),
      minted: Decimal::from_u128(new_info.supply).unwrap(),
      limit_per_mint: Decimal::from_u128(new_info.limit_per_mint).unwrap(),
      decimal: new_info.decimal,
      deploy_by: new_info.deploy_by.to_string(),
      deployed_number: new_info.deployed_number,
      deployed_timestamp: new_info.deployed_timestamp,
      latest_mint_number: new_info.latest_mint_number,
    };

    let mut err: Option<DbErr>;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let result = Brc20TokenWriter::create(&self.db, &token).await;
      match result {
        Ok(_) => err = None,
        Err(e) => err = Some(e),
      }
    });

    match err {
      None => Ok(()),
      Some(e) => Err(anyhow!("failed to get all token info! error: {e}")),
    }
  }

  fn update_mint_token_info(
    &mut self,
    tick: &Tick,
    minted_amt: u128,
    minted_block_number: u32,
  ) -> crate::Result<(), Self::Error> {
    let mut err: Option<DbErr>;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let result = Brc20TokenWriter::update_mint_info(
        &self.db,
        tick.as_str(),
        minted_amt,
        minted_block_number,
      )
      .await;
      match result {
        Ok(_) => err = None,
        Err(e) => err = Some(e),
      }
    });

    match err {
      None => Ok(()),
      Some(e) => Err(anyhow!("failed to get all token info! error: {e}")),
    }
  }

  fn save_transaction_receipts(
    &mut self,
    txid: &Txid,
    receipt: &[Receipt],
  ) -> crate::Result<(), Self::Error> {
    let recipets = vec![];
    for re in receipt.iter() {
      let mut recipet = Brc20TxReceipt {
        id: 0,
        tick: "".to_string(),
        inscription_id: re.inscription_id.to_string(),
        inscription_number: re.inscription_number,
        tx_id: txid.to_string(),
        old_satpoint: re.old_satpoint.to_string(),
        new_satpoint: re.new_satpoint.to_string(),
        op: re.op,
        from: re.from.to_string(),
        to: re.to.to_string(),
        amount: None,
        supply: None,
        limit_per_mint: None,
        decimal: None,
        msg: None,
      };
      match re.result {
        Ok(Deploy(deploy)) => {
          recipet.tick = deploy.tick.to_string();
          recipet.supply = Some(Decimal::from_u128(deploy.supply).unwrap());
          recipet.limit_per_mint = Some(Decimal::from_u128(deploy.limit_per_mint).unwrap());
          recipet.decimal = Some(deploy.decimal);
        }
        Ok(Event::Mint(mint)) => {
          recipet.tick = mint.tick.to_string();
          recipet.amount = Some(Decimal::from_u128(mint.amount).unwrap());
          recipet.msg = mint.msg;
        }
        Ok(Event::InscribeTransfer(mint)) => {
          recipet.tick = mint.tick.to_string();
          recipet.amount = Some(Decimal::from_u128(mint.amount).unwrap());
          recipet.msg = mint.msg;
        }
        Ok(Event::Transfer(mint)) => {
          recipet.tick = mint.tick.to_string();
          recipet.amount = Some(Decimal::from_u128(mint.amount).unwrap());
          recipet.msg = mint.msg;
        }
        Err(e) => return Err(anyhow!("event has err! {e}")),
      }
      recipets.push(recipet);
    }

    let mut err: Option<DbErr>;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let result = Brc20TxReceiptWriter::create(&self.db, &recipets).await;
      match result {
        Ok(_) => err = None,
        Err(e) => err = Some(e),
      }
    });

    match err {
      None => Ok(()),
      Some(e) => Err(anyhow!("failed to get all token info! error: {e}")),
    }
  }

  fn insert_transferable(
    &mut self,
    script: &ScriptKey,
    tick: &Tick,
    inscription: &TransferableLog,
  ) -> crate::Result<(), Self::Error> {
    let mut tx_log = Brc20TxableLog {
      id: 0,
      tick: inscription.tick.to_string(),
      inscription_id: inscription.inscription_id.to_string(),
      inscription_number: inscription.inscription_number,
      amount: Decimal::from_u128(inscription.amount).unwrap(),
      owner: inscription.owner.to_string(),
    };

    let mut err: Option<DbErr>;
    let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async move {
      let result = Brc20TxableLogWriter::create(&self.db, &tx_log).await;
      match result {
        Ok(_) => err = None,
        Err(e) => err = Some(e),
      }
    });

    match err {
      None => Ok(()),
      Some(e) => Err(anyhow!("failed to get all token info! error: {e}")),
    }
  }

  fn remove_transferable(
    &mut self,
    script: &ScriptKey,
    tick: &Tick,
    inscription_id: &InscriptionId,
  ) -> crate::Result<(), Self::Error> {
    // remove_transferable(self.BRC20_TRANSFERABLELOG, script, tick, inscription_id)
    Ok(())
  }

  fn insert_inscribe_transfer_inscription(
    &mut self,
    inscription_id: &InscriptionId,
    transfer_info: TransferInfo,
  ) -> crate::Result<(), Self::Error> {
    Ok(())

    // insert_inscribe_transfer_inscription(
    //   self.BRC20_INSCRIBE_TRANSFER,
    //   inscription_id,
    //   transfer_info,
    // )
  }

  fn remove_inscribe_transfer_inscription(
    &mut self,
    inscription_id: &InscriptionId,
  ) -> crate::Result<(), Self::Error> {
    Ok(())
    // remove_inscribe_transfer_inscription(self.BRC20_INSCRIBE_TRANSFER, inscription_id)
  }
}
