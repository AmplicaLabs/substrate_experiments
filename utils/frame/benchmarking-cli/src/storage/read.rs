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
use rand::prelude::*;
use std::{fmt::Debug, sync::Arc, time::Instant};

use super::cmd::StorageCmd;
use crate::shared::{new_rng, BenchRecord};

impl StorageCmd {
	/// Benchmarks the time it takes to read a single Storage item.
	/// Uses the latest state that is available for the given client.
	pub(crate) fn bench_read<B, BA, C>(&self, client: Arc<C>) -> Result<BenchRecord>
	where
		C: UsageProvider<B> + StorageProvider<B, BA>,
		B: BlockT + Debug,
		BA: ClientBackend<B>,
		<<B as BlockT>::Header as HeaderT>::Number: From<u32>,
	{
		let mut record = BenchRecord::default();
		let block = BlockId::Number(client.usage_info().chain.best_number);

		info!("Preparing keys from block {}", block);
		info!("Reading {} threshold", self.params.read_threshold);
		let (mut rng, _) = new_rng(None);
		let mut sampled_keys = Vec::new();

		let mut count = 0u32;
		for key in client.storage_keys_iter(&block, None, None)? {
			match (self.params.include_child_trees, self.is_child_key(key.clone().0)) {
				(true, Some(info)) => {
					// child tree key
					let mut first = true;
					for ck in client.child_storage_keys_iter(&block, info.clone(), None, None)? {
						if first ||  rng.gen_range(1..=100) <= self.params.read_threshold{
							first = false;
							sampled_keys.push((ck, Some(info.clone())));
						}
					}
				},
				_ => sampled_keys.push((key, None))
			}

			count += 1;
			if count % 10_000 == 0 {
				info!("Read {}", count);
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
					record.append(v.0.len(), start.elapsed())?;
				},
				_ => {
					let start = Instant::now();
					let v = client
						.storage(&block, &key)
						.expect("Checked above to exist")
						.ok_or("Value unexpectedly empty")?;
					record.append(v.0.len(), start.elapsed())?;
				}
			}

			count += 1;
			if count % 10_000 == 0 {
				info!("benchmarked {}", count);
			}
		}

		Ok(record)
	}
}
