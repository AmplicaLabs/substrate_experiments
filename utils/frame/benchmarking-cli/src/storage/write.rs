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

use log::{info, trace};
use min_max_heap::MinMaxHeap;
use rand::prelude::*;
use sc_cli::Result;
use sc_client_api::{Backend as ClientBackend, StorageProvider, UsageProvider};
use sc_client_db::{DbHash, DbState};
use sp_api::StateBackend;
use sp_blockchain::HeaderBackend;
use sp_database::{ColumnId, Transaction};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, HashFor, Header as HeaderT},
};
use sp_state_machine::backend::Sampling;
use sp_storage::{ChildInfo, StateVersion};
use sp_trie::PrefixedMemoryDB;
use std::{
	fmt::Debug,
	sync::Arc,
	time::{Duration, Instant},
};

use super::cmd::StorageCmd;
use crate::{
	shared::{new_rng, BenchRecord},
	storage::cmd::BenchNode,
};

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
		let mut max_heap = MinMaxHeap::<BenchNode>::with_capacity(self.params.outliers_size);
		// Store the time that it took to write each value.
		let mut record = BenchRecord::default();

		let block = BlockId::Number(client.usage_info().chain.best_number);
		let header = client.header(block)?.ok_or("Header not found")?;
		let original_root = *header.state_root();
		let trie = DbState::<Block>::new(storage.clone(), original_root);

		info!("Preparing keys from block {}", block);
		// Load all KV pairs and randomly shuffle them.
		let (mut rng, _) = new_rng(None);
		let kvs = trie.pairs_limit(&mut rng, self.params.write_threshold.try_into().unwrap());
		info!("pairs are ready len={}", kvs.len());

		let mut sampled_keys = Vec::with_capacity(kvs.len());
		// Generate all random values first; Make sure there are no collisions with existing
		// db entries, so we can rollback all additions without corrupting existing entries.
		for (k, original_v) in kvs {
			match (self.params.include_child_trees, self.is_child_key(k.to_vec())) {
				(true, Some(info)) => {
					 let mut first = true;
					 for ck in client.child_storage_keys_iter(&block, info.clone(), None, None)? {
						if first || rng.gen_range(1..=100) <= self.params.write_threshold	 {
							first = false;
							sampled_keys.push((ck.0, 4096, Some(info.clone()))); // TODO hardcoded
						}
					 }
				},
				_ => sampled_keys.push((k.into(), original_v.len(), None)),
			}
		}

		info!("Writing {} sampled keys", sampled_keys.len());
		sampled_keys.shuffle(&mut rng);
		info!("shuffle done");

		let mut count = 0u64;
		// Write each value in one commit.
		for (k, sz, option) in sampled_keys {
			let mut new_v = vec![0; sz];
			let mut repeats = 3;
			while repeats > 0 {
				// Create a random value to overwrite with.
				// NOTE: We use a possibly higher entropy than the original value,
				// could be improved but acts as an over-estimation which is fine for now.
				rng.fill_bytes(&mut new_v[..]);
				if check_new_value::<Block>(
					db.clone(),
					&trie,
					&k,
					&new_v,
					self.state_version(),
					state_col,
					option.clone(),
				) {
					break
				}
				repeats -= 1;
			}

			if repeats == 0 {
				info!("failed {:x?}", k.clone());
				continue
			}

			// Write each value in one commit.
			let (size, duration) = measure_write::<Block>(
				db.clone(),
				&trie,
				k.to_vec(),
				new_v.to_vec(),
				self.state_version(),
				state_col,
				option.clone(),
			)?;
			record.append(size, duration)?;

			max_heap.push(BenchNode {
				d: duration,
				sz: size,
				key: k.to_vec(),
				info: option.clone(),
			});

			if max_heap.len() > self.params.outliers_size {
				max_heap.pop_min();
			}

			count += 1;
			if count % 10_000 == 0 {
				info!("written {}", count)
			}
		}

		while !max_heap.is_empty() {
			info!("{:?}", max_heap.pop_max());
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

/// Measures write benchmark
/// if `child_info` exist then it means this is a child tree key
fn measure_write<Block: BlockT>(
	db: Arc<dyn sp_database::Database<DbHash>>,
	trie: &DbState<Block>,
	key: Vec<u8>,
	new_v: Vec<u8>,
	version: StateVersion,
	col: ColumnId,
	child_info: Option<ChildInfo>,
) -> Result<(usize, Duration)> {
	let start = Instant::now();
	// Create a TX that will modify the Trie in the DB and
	// calculate the root hash of the Trie after the modification.
	let replace = vec![(key.as_ref(), Some(new_v.as_ref()))];
	let stx = match child_info {
		Some(info) => trie.child_storage_root(&info, replace.iter().cloned(), version).2,
		None => trie.storage_root(replace.iter().cloned(), version).1,
	};
	// Only the keep the insertions, since we do not want to benchmark pruning.
	let tx = convert_tx::<Block>(db.clone(), stx.clone(), false, col);
	db.commit(tx).map_err(|e| format!("Writing to the Database: {}", e))?;
	let result = (new_v.len(), start.elapsed());

	// Now undo the changes by removing what was added.
	let tx = convert_tx::<Block>(db.clone(), stx.clone(), true, col);
	db.commit(tx).map_err(|e| format!("Writing to the Database: {}", e))?;
	Ok(result)
}

/// Checks if a new value causes any collision in tree updates
/// returns true if there is no collision
/// if `child_info` exist then it means this is a child tree key
fn check_new_value<Block: BlockT>(
	db: Arc<dyn sp_database::Database<DbHash>>,
	trie: &DbState<Block>,
	key: &Vec<u8>,
	new_v: &Vec<u8>,
	version: StateVersion,
	col: ColumnId,
	child_info: Option<ChildInfo>,
) -> bool {
	let new_kv = vec![(key.as_ref(), Some(new_v.as_ref()))];
	let mut stx = match child_info {
		Some(info) => trie.child_storage_root(&info, new_kv.iter().cloned(), version).2,
		None => trie.storage_root(new_kv.iter().cloned(), version).1,
	};
	for (mut k, (_, rc)) in stx.drain().into_iter() {
		if rc > 0 {
			db.sanitize_key(&mut k);
			if db.get(col, &k).is_some() {
				trace!("Benchmark-store key creation: Key collision detected, retry");
				return false
			}
		}
	}
	true
}
