// This file is part of Substrate.

// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use frame_support::storage::child::ChildInfo;
use log::{debug, info, trace};
use rand::prelude::*;
use sc_cli::Result;
use sc_client_api::{Backend as ClientBackend, StorageProvider, UsageProvider};
use sc_client_db::{DbHash, DbState};
use sp_api::StateBackend;
use sp_blockchain::HeaderBackend;
use sp_core::storage::StorageKey;
use sp_database::{ColumnId, Transaction};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, HashFor, Header as HeaderT},
};
use sp_state_machine::backend::Sampling;
use sp_trie::PrefixedMemoryDB;
use std::{fmt::Debug, sync::Arc, time::Instant};

use super::cmd::StorageCmd;
use crate::shared::{new_rng, BenchRecord};

impl StorageCmd {
	/// Benchmarks the time it takes to write a single Storage item.
	/// Uses the latest state that is available for the given client.
	pub(crate) fn bench_write<Block, BA, H, C>(
		&self,
		client: Arc<C>,
		(db, state_col): (Arc<dyn sp_database::Database<DbHash>>, ColumnId),
		storage: Arc<dyn sp_state_machine::Storage<HashFor<Block>>>,
	) -> Result<BenchRecord>
	where
		Block: BlockT<Header = H, Hash = DbHash> + Debug,
		H: HeaderT<Hash = DbHash>,
		BA: ClientBackend<Block>,
		C: UsageProvider<Block> + HeaderBackend<Block> + StorageProvider<Block, BA>,
	{
		// Store the time that it took to write each value.
		let mut record = BenchRecord::default();

		let block = BlockId::Number(client.usage_info().chain.best_number);
		let header = client.header(block)?.ok_or("Header not found")?;
		let original_root = *header.state_root();
		let trie = DbState::<Block>::new(storage.clone(), original_root);

		info!("Preparing keys from block {}", block);
		// Load all KV pairs and randomly shuffle them.
		let (mut rng, _) = new_rng(None);
		let mut kvs = trie.pairs_limit(&mut rng, self.params.write_threshold.try_into().unwrap());
		info!("pairs are ready len={}", kvs.len());

		kvs.shuffle(&mut rng);
		info!("shuffle done");

		let empty_prefix = StorageKey(Vec::new());
		let child_prefix = ":child_storage:default:".as_bytes();
		let mut c_kv: Vec<(ChildInfo, StorageKey, Vec<u8>)> = Vec::new();
		// Generate all random values first; Make sure there are no collisions with existing
		// db entries, so we can rollback all additions without corrupting existing entries.
		for (k, original_v) in kvs.iter_mut() {
			if k.clone().starts_with(child_prefix) {
				continue
			}

			'retry: loop {
				let mut new_v = vec![0; original_v.len()];
				// Create a random value to overwrite with.
				// NOTE: We use a possibly higher entropy than the original value,
				// could be improved but acts as an over-estimation which is fine for now.
				rng.fill_bytes(&mut new_v[..]);
				let new_kv = vec![(k.as_ref(), Some(new_v.as_ref()))];
				let (_, mut stx) = trie.storage_root(new_kv.iter().cloned(), self.state_version());
				for (mut k, (_, rc)) in stx.drain().into_iter() {
					if rc > 0 {
						db.sanitize_key(&mut k);
						if db.get(state_col, &k).is_some() {
							trace!("Benchmark-store key creation: Key collision detected, retry");
							continue 'retry
						}
					}
				}
				*original_v = new_v;
				break
			}
		}

		info!("Writing {} keys", kvs.len());
		let mut count = 0u64;
		// Write each value in one commit.
		for (k, new_v) in kvs.iter() {
			if k.clone().starts_with(child_prefix) {
				debug!("root {:?}", hex::encode(k.clone()));
				let trie_id = k.strip_prefix(child_prefix);
				let info = ChildInfo::new_default(trie_id.unwrap());
				let my_keys = client.child_storage_keys(&block, &info.clone(), &empty_prefix)?;
				for kk in my_keys {
					let rand = rng.gen_range(1..=100);
					if rand <= self.params.write_threshold {
						// let v = client
						// 	.child_storage(&block, &info.clone(), &kk)
						// 	.expect("Checked above to exist")
						// 	.ok_or("Value unexpectedly empty")?;
						c_kv.push((info.clone(), kk.clone(), vec![0, 1]));
						debug!("-> {:?}", hex::encode(kk.clone()));
					}
				}
			} else {
				// Interesting part here:
				let start = Instant::now();
				// Create a TX that will modify the Trie in the DB and
				// calculate the root hash of the Trie after the modification.
				let replace = vec![(k.as_ref(), Some(new_v.as_ref()))];
				let (_, stx) = trie.storage_root(replace.iter().cloned(), self.state_version());
				// Only the keep the insertions, since we do not want to benchmark pruning.
				let tx = convert_tx::<Block>(db.clone(), stx.clone(), false, state_col);
				db.commit(tx).map_err(|e| format!("Writing to the Database: {}", e))?;
				record.append(new_v.len(), start.elapsed())?;

				// Now undo the changes by removing what was added.
				let tx = convert_tx::<Block>(db.clone(), stx.clone(), true, state_col);
				db.commit(tx).map_err(|e| format!("Writing to the Database: {}", e))?;
			}

			count += 1;
			if count % 100_000 == 0 {
				info!("{}", count)
			}
		}

		info!("Write {} child keys", c_kv.len());
		c_kv.shuffle(&mut rng);
		info!("shuffle done");

		for (info, k, original_v) in c_kv.iter_mut() {
			debug!(
				"info={:?}  k={:?}  v={:?}",
				hex::encode(info.keyspace()),
				hex::encode(k.clone()),
				hex::encode(original_v.clone())
			);
			'retry_again: loop {
				let mut new_v = vec![0; original_v.len()];
				// Create a random value to overwrite with.
				// NOTE: We use a possibly higher entropy than the original value,
				// could be improved but acts as an over-estimation which is fine for now.
				rng.fill_bytes(&mut new_v[..]);
				let new_kv = vec![(k.as_ref(), Some(new_v.as_ref()))];
				let (_, _, mut stx) =
					trie.child_storage_root(&info, new_kv.iter().cloned(), self.state_version());
				for (mut k, (_, rc)) in stx.drain().into_iter() {
					debug!("drain -> {:?}", hex::encode(k.clone()));
					if rc > 0 {
						db.sanitize_key(&mut k);
						if db.get(state_col, &k).is_some() {
							trace!("Benchmark-store key creation: Key collision detected, retry");
							continue 'retry_again
						}
					}
				}
				*original_v = new_v;
				break
			}
		}

		info!("child benchmarks starting..");
		// Write each value in one commit.
		for (info, k, new_v) in c_kv.iter() {
			// Interesting part here:
			let start = Instant::now();
			// Create a TX that will modify the Trie in the DB and
			// calculate the root hash of the Trie after the modification.
			let replace = vec![(k.as_ref(), Some(new_v.as_ref()))];
			let (_, _, stx) =
				trie.child_storage_root(info, replace.iter().cloned(), self.state_version());
			// Only the keep the insertions, since we do not want to benchmark pruning.
			let tx = convert_tx::<Block>(db.clone(), stx.clone(), false, state_col);
			db.commit(tx).map_err(|e| format!("Writing to the Database: {}", e))?;
			record.append(new_v.len(), start.elapsed())?;

			// Now undo the changes by removing what was added.
			let tx = convert_tx::<Block>(db.clone(), stx.clone(), true, state_col);
			db.commit(tx).map_err(|e| format!("Writing to the Database: {}", e))?;

			count += 1;
			if count % 100_000 == 0 {
				info!("{}", count)
			}
		}

		Ok(record)
	}
}

/// Converts a Trie transaction into a DB transaction.
/// Removals are ignored and will not be included in the final tx.
/// `invert_inserts` replaces all inserts with removals.
fn convert_tx<B: BlockT>(
	db: Arc<dyn sp_database::Database<DbHash>>,
	mut tx: PrefixedMemoryDB<HashFor<B>>,
	invert_inserts: bool,
	col: ColumnId,
) -> Transaction<DbHash> {
	let mut ret = Transaction::<DbHash>::default();

	for (mut k, (v, rc)) in tx.drain().into_iter() {
		if rc > 0 {
			db.sanitize_key(&mut k);
			if invert_inserts {
				ret.remove(col, &k);
			} else {
				ret.set(col, &k, &v);
			}
		}
		// < 0 means removal - ignored.
		// 0 means no modification.
	}
	ret
}
