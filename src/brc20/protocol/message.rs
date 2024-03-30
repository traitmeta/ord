use crate::brc20::datastore as brc20_store;
use crate::brc20::protocol::brc20 as brc20_proto;

#[allow(clippy::upper_case_acronyms)]
pub enum Message {
  BRC20(brc20_proto::Message),
}

#[allow(clippy::upper_case_acronyms)]
#[allow(unused)]
pub enum Receipt {
  BRC20(brc20_store::Receipt),
}
