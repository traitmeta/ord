use super::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MyResponse<T> {
  pub code: u64,
  pub message: String,
  pub data: T,
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RuneResp {
  pub rune: String,
  pub spacers: u32,
  pub commitment: String,
  pub can_etch: bool,
}

