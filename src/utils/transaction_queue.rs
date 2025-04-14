// use std::collections::VecDeque;
// use std::sync::{Arc, Mutex};
// use subxt::{OnlineClient, PolkadotConfig};
// use subxt_signer::sr25519::Keypair;
// use subxt::utils::H256;
// use crate::substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;

// #[derive(Debug)]
// enum Transaction {
//     SubmitResult {
//         completed_hash: H256,
//         result_cid: BoundedVec<u8>,
//         task_id: u64,
//     },
//     SubmitResultVerification {
//         completed_hash: H256,
//         task_id: u64,
//     },
//     SubmitResultResolution {
//         completed_hash: H256,
//         task_id: u64,
//     },
// }

// #[derive(Clone)]
// pub struct TransactionQueue {
//     inner: Arc<Mutex<VecDeque<Transaction>>>,
// }

// impl TransactionQueue {
//     pub fn new() -> Self {
//         Self {
//             inner: Arc::new(Mutex::new(VecDeque::new())),
//         }
//     }

//     pub fn add_submit_result(
//         &self,
//         completed_hash: H256,
//         result_cid: BoundedVec<u8>,
//         task_id: u64,
//     ) {
//         let mut queue = self.inner.lock().unwrap();
//         queue.push_back(Transaction::SubmitResult {
//             completed_hash,
//             result_cid,
//             task_id,
//         });
//     }

//     pub fn add_submit_verification(&self, completed_hash: H256, task_id: u64) {
//         let mut queue = self.inner.lock().unwrap();
//         queue.push_back(Transaction::SubmitResultVerification {
//             completed_hash,
//             task_id,
//         });
//     }

//     pub fn add_submit_resolution(&self, completed_hash: H256, task_id: u64) {
//         let mut queue = self.inner.lock().unwrap();
//         queue.push_back(Transaction::SubmitResultResolution {
//             completed_hash,
//             task_id,
//         });
//     }

//     pub async fn process_next(
//         &self,
//         api: &OnlineClient<PolkadotConfig>,
//         signer_keypair: &Keypair,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         let transaction = {
//             let mut queue = self.inner.lock().unwrap();
//             queue.pop_front()
//         };

//         if let Some(transaction) = transaction {
//             match transaction {
//                 Transaction::SubmitResult {
//                     completed_hash,
//                     result_cid,
//                     task_id,
//                 } => {
//                     submit_result_internal(api, signer_keypair, completed_hash, result_cid, task_id)
//                         .await?;
//                 }
//                 Transaction::SubmitResultVerification {
//                     completed_hash,
//                     task_id,
//                 } => {
//                     submit_result_verification_internal(api, signer_keypair, completed_hash, task_id)
//                         .await?;
//                 }
//                 Transaction::SubmitResultResolution {
//                     completed_hash,
//                     task_id,
//                 } => {
//                     submit_result_resolution_internal(api, signer_keypair, completed_hash, task_id)
//                         .await?;
//                 }
//             }
//         }

//         Ok(())
//     }
// }