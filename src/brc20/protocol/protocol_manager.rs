use crate::brc20::datastore::ord::OrdReaderWriter;
use crate::brc20::protocol::context::Context;
use {
  super::*,
  crate::{
    index::updater::BlockData,
    brc20::datastore::ord::operation::InscriptionOp,
    Instant, Result,
  },
  bitcoin::Txid,
  std::collections::HashMap,
};

pub struct ProtocolManager {
  config: ProtocolConfig,
  call_man: CallManager,
  resolve_man: MsgResolveManager,
}

impl ProtocolManager {
  // Need three datastore, and they're all in the same write transaction.
  pub fn new(config: ProtocolConfig) -> Self {
    Self {
      config,
      call_man: CallManager::new(),
      resolve_man: MsgResolveManager::new(config),
    }
  }

  pub(crate) fn index_block(
    &self,
    context: &mut Context,
    block: &BlockData,
    operations: HashMap<Txid, Vec<InscriptionOp>>,
  ) -> Result {
    let start = Instant::now();
    let mut inscriptions_size = 0;
    let mut messages_size = 0;
    let mut cost1 = 0u128;
    let mut cost2 = 0u128;
    let mut cost3 = 0u128;
    // skip the coinbase transaction.
    for (tx, txid) in block.txdata.iter() {
      // skip coinbase transaction.
      if tx
        .input
        .first()
        .is_some_and(|tx_in| tx_in.previous_output.is_null())
      {
        continue;
      }

      // index inscription operations.
      if let Some(tx_operations) = operations.get(txid) {
        // save all transaction operations to ord database.
        if self.config.enable_ord_receipts
          && context.chain.blockheight >= self.config.first_inscription_height
        {
          let start = Instant::now();
          context.save_transaction_operations(txid, tx_operations)?;
          inscriptions_size += tx_operations.len();
          cost1 += start.elapsed().as_micros();
        }

        let start = Instant::now();
        // Resolve and execute messages.
        let messages = self
          .resolve_man
          .resolve_message(context, tx, tx_operations)?;
        cost2 += start.elapsed().as_micros();

        let start = Instant::now();
        self.call_man.execute_message(context, txid, &messages)?;
        cost3 += start.elapsed().as_micros();
        messages_size += messages.len();
      }
    }

    log::info!(
      "Protocol Manager indexed block {} with ord inscriptions {}, messages {} in {} ms, {}/{}/{}",
      context.chain.blockheight,
      inscriptions_size,
      messages_size,
      start.elapsed().as_millis(),
      cost1/1000,
      cost2/1000,
      cost3/1000,
    );
    Ok(())
  }
}
