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

use sc_cli::Result;
use sc_client_api::{Backend as ClientBackend, StorageProvider, UsageProvider};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, Header as HeaderT},
};

use log::info;
use min_max_heap::MinMaxHeap;
use rand::prelude::*;
use sp_storage::StorageKey;
use std::{fmt::Debug, sync::Arc, time::Instant};

use super::cmd::StorageCmd;
use crate::{
	shared::{new_rng, BenchRecord},
	storage::cmd::BenchNode,
};

impl StorageCmd {
	/// Benchmarks the time it takes to read a single Storage item.
	/// Uses the latest state that is available for the given client.
	pub(crate) fn bench_read<B, BA, C>(
		&self,
		client: Arc<C>,
		keys: Vec<StorageKey>,
	) -> Result<BenchRecord>
	where
		C: UsageProvider<B> + StorageProvider<B, BA>,
		B: BlockT + Debug,
		BA: ClientBackend<B>,
		<<B as BlockT>::Header as HeaderT>::Number: From<u32>,
	{
		let mut max_heap = MinMaxHeap::<BenchNode>::with_capacity(self.params.outliers_size);
		let mut record = BenchRecord::default();
		let block = BlockId::Number(client.usage_info().chain.best_number);

		info!("Preparing keys from block {}", block);
		info!("Reading {} keys with {} threshold", keys.len(), self.params.read_threshold);
		let (mut rng, _) = new_rng(None);
		let mut sampled_keys = Vec::new();

		let mut count = 0u32;
		for key in keys {
			if rng.gen_range(1..=100) <= self.params.read_threshold {
				match (self.params.include_child_trees, self.is_child_key(key.clone().0)) {
					(true, Some(info)) => {
						// child tree key
						sampled_keys.push((StorageKey(hex::decode("e8030000").unwrap()), Some(info.clone())));
						// let mut first = true;
						// for ck in
						// 	client.child_storage_keys_iter(&block, info.clone(), None, None)?
						// {
						// 	if first || rng.gen_range(1..=100) <= self.params.read_threshold {
						// 		first = false;
						// 		sampled_keys.push((ck.clone(), Some(info.clone())));
						// 	}
						// }
					},
					_ => sampled_keys.push((key.clone(), None)),
				}
			}

			count += 1;
			if count % 10_000 == 0 {
				info!("Read {} sampled {}", count, sampled_keys.len());
			}
		}

		info!("sampled {} keys", sampled_keys.len());
		sampled_keys.shuffle(&mut rng);

		count = 0;
		for (key, option) in sampled_keys {
			match (self.params.include_child_trees, option) {
				(true, Some(info)) => {
					let start = Instant::now();
					let v = client
						.child_storage(&block, &info, &key)
						.expect("Checked above to exist")
						.ok_or("Value unexpectedly empty")?;
					let duration = start.elapsed();
					record.append(v.0.len(), duration)?;

					max_heap.push(BenchNode {
						d: duration,
						sz: v.0.len(),
						key: key.clone().0,
						info: Some(info.clone()),
					});
				},
				_ => {
					let start = Instant::now();
					let v = client
						.storage(&block, &key)
						.expect("Checked above to exist")
						.ok_or("Value unexpectedly empty")?;
					let duration = start.elapsed();
					record.append(v.0.len(), duration)?;

					max_heap.push(BenchNode {
						d: duration,
						sz: v.0.len(),
						key: key.clone().0,
						info: None,
					});
				},
			}

			if max_heap.len() > self.params.outliers_size {
				max_heap.pop_min();
			}

			count += 1;
			if count % 10_000 == 0 {
				info!("benchmarked {}", count);
			}
		}

		while !max_heap.is_empty() {
			info!("{:?}", max_heap.pop_max());
		}

		Ok(record)
	}
}
