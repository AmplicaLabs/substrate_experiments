use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{pallet_prelude::ConstU32, BoundedVec, RuntimeDebug};
use scale_info::TypeInfo;
use sp_std::{
	cmp::{Ord, Ordering},
	prelude::*,
};
/// message source id
pub type MessageSourceId = u64;
/// account if type
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
/// storage key type
pub type StorageKey = BoundedVec<u8, ConstU32<4096>>;
/// Public Page type
pub type PublicPage = BoundedVec<MessageSourceId, ConstU32<512>>;
/// Private Page type
pub type PrivatePage = BoundedVec<u8, ConstU32<4096>>;

/// graph edge
#[derive(Clone, Copy, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, Eq, PartialOrd)]
#[scale_info(skip_type_params(T))]
pub struct Edge {
	/// target node id
	pub static_id: MessageSourceId,
	/// connection permission
	pub permission: Permission,
}

impl Ord for Edge {
	fn cmp(&self, other: &Self) -> Ordering {
		self.static_id.cmp(&other.static_id)
	}
}

/// graph node
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Node {}

/// connection permission
#[derive(Clone, Copy, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, Ord, Eq, PartialOrd)]
pub struct Permission {
	/// permission type
	pub data: u16,
}

/// graph key
#[derive(Clone, Copy, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, Ord, Eq, PartialOrd)]
pub struct GraphKey {
	/// permission type
	pub permission: Permission,
	/// page number
	pub page: u16,
}

/// type of graph
#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq)]
pub enum GraphType {
	/// public graph
	Public,
	/// private graph
	Private,
}

pub static EXPONENTIAL_DISTRO_PAGES: [u16; 1000] = [
	18u16, 33u16, 13u16, 22u16, 19u16, 28u16, 25u16, 31u16, 3u16, 10u16, 16u16, 1u16, 3u16, 8u16,
	14u16, 10u16, 2u16, 1u16, 3u16, 8u16, 19u16, 1u16, 17u16, 32u16, 5u16, 8u16, 49u16, 23u16,
	3u16, 18u16, 2u16, 10u16, 23u16, 21u16, 15u16, 8u16, 22u16, 11u16, 4u16, 12u16, 10u16, 4u16,
	1u16, 1u16, 1u16, 3u16, 8u16, 11u16, 11u16, 10u16, 1u16, 1u16, 4u16, 12u16, 17u16, 1u16, 17u16,
	9u16, 5u16, 32u16, 3u16, 1u16, 28u16, 4u16, 1u16, 1u16, 4u16, 2u16, 4u16, 13u16, 12u16, 30u16,
	10u16, 3u16, 24u16, 9u16, 3u16, 1u16, 12u16, 4u16, 1u16, 15u16, 16u16, 10u16, 26u16, 16u16,
	3u16, 16u16, 6u16, 2u16, 9u16, 28u16, 35u16, 18u16, 8u16, 5u16, 1u16, 8u16, 4u16, 8u16, 7u16,
	3u16, 18u16, 1u16, 1u16, 2u16, 9u16, 46u16, 25u16, 4u16, 14u16, 22u16, 2u16, 35u16, 23u16,
	2u16, 15u16, 1u16, 1u16, 4u16, 19u16, 1u16, 4u16, 2u16, 1u16, 2u16, 17u16, 6u16, 2u16, 6u16,
	8u16, 5u16, 4u16, 6u16, 31u16, 1u16, 7u16, 1u16, 3u16, 3u16, 5u16, 13u16, 5u16, 2u16, 4u16,
	5u16, 18u16, 32u16, 28u16, 4u16, 6u16, 7u16, 18u16, 6u16, 3u16, 10u16, 19u16, 3u16, 13u16,
	9u16, 24u16, 3u16, 3u16, 1u16, 9u16, 26u16, 11u16, 5u16, 41u16, 5u16, 3u16, 2u16, 4u16, 11u16,
	23u16, 5u16, 1u16, 9u16, 3u16, 6u16, 22u16, 10u16, 3u16, 13u16, 1u16, 19u16, 3u16, 3u16, 3u16,
	1u16, 11u16, 3u16, 3u16, 2u16, 5u16, 7u16, 13u16, 2u16, 6u16, 2u16, 8u16, 24u16, 1u16, 10u16,
	4u16, 29u16, 7u16, 20u16, 22u16, 9u16, 8u16, 11u16, 1u16, 4u16, 11u16, 32u16, 4u16, 6u16,
	15u16, 6u16, 29u16, 11u16, 7u16, 5u16, 9u16, 18u16, 15u16, 15u16, 26u16, 8u16, 7u16, 15u16,
	28u16, 20u16, 4u16, 39u16, 5u16, 1u16, 3u16, 21u16, 6u16, 7u16, 11u16, 6u16, 2u16, 8u16, 2u16,
	4u16, 3u16, 1u16, 6u16, 13u16, 5u16, 7u16, 7u16, 24u16, 10u16, 20u16, 7u16, 21u16, 20u16, 4u16,
	4u16, 3u16, 1u16, 19u16, 9u16, 16u16, 1u16, 11u16, 6u16, 9u16, 7u16, 2u16, 7u16, 6u16, 4u16,
	3u16, 1u16, 26u16, 7u16, 12u16, 10u16, 16u16, 18u16, 21u16, 37u16, 13u16, 3u16, 3u16, 15u16,
	8u16, 5u16, 7u16, 11u16, 22u16, 2u16, 15u16, 22u16, 11u16, 13u16, 2u16, 5u16, 3u16, 20u16,
	19u16, 59u16, 2u16, 7u16, 10u16, 2u16, 6u16, 15u16, 6u16, 20u16, 16u16, 41u16, 3u16, 4u16,
	5u16, 12u16, 7u16, 47u16, 48u16, 1u16, 8u16, 18u16, 33u16, 7u16, 3u16, 5u16, 6u16, 6u16, 6u16,
	4u16, 9u16, 37u16, 2u16, 14u16, 10u16, 8u16, 11u16, 4u16, 8u16, 4u16, 9u16, 10u16, 1u16, 9u16,
	11u16, 29u16, 5u16, 3u16, 26u16, 43u16, 14u16, 8u16, 13u16, 13u16, 4u16, 2u16, 2u16, 7u16,
	3u16, 13u16, 4u16, 10u16, 6u16, 7u16, 4u16, 3u16, 7u16, 1u16, 2u16, 13u16, 17u16, 4u16, 3u16,
	18u16, 7u16, 12u16, 5u16, 7u16, 23u16, 4u16, 7u16, 4u16, 7u16, 10u16, 3u16, 9u16, 1u16, 1u16,
	24u16, 3u16, 11u16, 5u16, 3u16, 4u16, 13u16, 34u16, 12u16, 37u16, 20u16, 29u16, 1u16, 4u16,
	3u16, 18u16, 4u16, 12u16, 30u16, 3u16, 1u16, 8u16, 1u16, 1u16, 3u16, 5u16, 1u16, 36u16, 21u16,
	7u16, 16u16, 5u16, 13u16, 2u16, 2u16, 21u16, 10u16, 2u16, 26u16, 31u16, 11u16, 12u16, 22u16,
	4u16, 2u16, 1u16, 5u16, 4u16, 3u16, 10u16, 72u16, 27u16, 2u16, 22u16, 3u16, 9u16, 17u16, 13u16,
	16u16, 1u16, 3u16, 5u16, 2u16, 6u16, 5u16, 3u16, 57u16, 23u16, 3u16, 3u16, 1u16, 31u16, 20u16,
	1u16, 18u16, 1u16, 8u16, 7u16, 9u16, 2u16, 2u16, 9u16, 1u16, 5u16, 10u16, 5u16, 14u16, 11u16,
	5u16, 11u16, 7u16, 1u16, 9u16, 12u16, 12u16, 6u16, 15u16, 10u16, 2u16, 16u16, 5u16, 8u16, 1u16,
	24u16, 14u16, 18u16, 4u16, 47u16, 12u16, 2u16, 6u16, 13u16, 8u16, 1u16, 13u16, 28u16, 17u16,
	10u16, 20u16, 8u16, 1u16, 3u16, 21u16, 17u16, 9u16, 5u16, 7u16, 47u16, 16u16, 40u16, 2u16,
	3u16, 10u16, 15u16, 5u16, 7u16, 5u16, 3u16, 3u16, 2u16, 8u16, 1u16, 14u16, 54u16, 4u16, 20u16,
	4u16, 4u16, 25u16, 4u16, 2u16, 19u16, 21u16, 19u16, 4u16, 10u16, 1u16, 8u16, 1u16, 36u16,
	20u16, 4u16, 5u16, 3u16, 4u16, 60u16, 7u16, 6u16, 6u16, 20u16, 11u16, 9u16, 7u16, 1u16, 3u16,
	11u16, 15u16, 6u16, 55u16, 11u16, 6u16, 36u16, 7u16, 8u16, 24u16, 36u16, 3u16, 9u16, 4u16,
	13u16, 2u16, 4u16, 9u16, 4u16, 16u16, 3u16, 1u16, 4u16, 8u16, 15u16, 8u16, 2u16, 5u16, 46u16,
	20u16, 9u16, 6u16, 13u16, 3u16, 10u16, 43u16, 10u16, 16u16, 1u16, 17u16, 4u16, 7u16, 2u16,
	6u16, 1u16, 11u16, 12u16, 20u16, 4u16, 4u16, 8u16, 12u16, 10u16, 2u16, 4u16, 40u16, 29u16,
	12u16, 25u16, 13u16, 13u16, 34u16, 17u16, 8u16, 12u16, 6u16, 19u16, 10u16, 17u16, 1u16, 26u16,
	1u16, 31u16, 6u16, 29u16, 15u16, 5u16, 3u16, 8u16, 7u16, 9u16, 47u16, 13u16, 4u16, 1u16, 10u16,
	2u16, 3u16, 1u16, 26u16, 21u16, 15u16, 25u16, 27u16, 5u16, 19u16, 4u16, 1u16, 29u16, 12u16,
	5u16, 43u16, 2u16, 18u16, 7u16, 3u16, 1u16, 4u16, 27u16, 20u16, 7u16, 1u16, 24u16, 3u16, 20u16,
	40u16, 26u16, 5u16, 7u16, 6u16, 17u16, 8u16, 11u16, 18u16, 10u16, 19u16, 12u16, 12u16, 4u16,
	9u16, 24u16, 9u16, 3u16, 11u16, 6u16, 10u16, 17u16, 97u16, 9u16, 14u16, 21u16, 10u16, 16u16,
	3u16, 15u16, 14u16, 1u16, 1u16, 16u16, 5u16, 11u16, 3u16, 1u16, 11u16, 5u16, 8u16, 17u16, 4u16,
	3u16, 7u16, 2u16, 7u16, 4u16, 5u16, 4u16, 6u16, 12u16, 8u16, 10u16, 4u16, 3u16, 2u16, 16u16,
	3u16, 5u16, 1u16, 3u16, 9u16, 31u16, 9u16, 10u16, 3u16, 7u16, 9u16, 10u16, 6u16, 9u16, 6u16,
	6u16, 7u16, 27u16, 3u16, 17u16, 1u16, 2u16, 1u16, 11u16, 5u16, 14u16, 10u16, 4u16, 4u16, 3u16,
	16u16, 18u16, 43u16, 2u16, 6u16, 1u16, 4u16, 3u16, 9u16, 3u16, 28u16, 11u16, 15u16, 16u16,
	8u16, 2u16, 29u16, 8u16, 4u16, 7u16, 13u16, 2u16, 2u16, 6u16, 2u16, 2u16, 35u16, 4u16, 10u16,
	4u16, 5u16, 10u16, 10u16, 7u16, 7u16, 26u16, 24u16, 5u16, 2u16, 7u16, 5u16, 6u16, 2u16, 6u16,
	9u16, 7u16, 2u16, 4u16, 8u16, 3u16, 25u16, 7u16, 44u16, 5u16, 3u16, 10u16, 19u16, 4u16, 8u16,
	23u16, 2u16, 3u16, 10u16, 1u16, 5u16, 19u16, 40u16, 5u16, 26u16, 10u16, 22u16, 27u16, 5u16,
	4u16, 31u16, 2u16, 3u16, 7u16, 18u16, 43u16, 13u16, 29u16, 2u16, 2u16, 40u16, 5u16, 19u16,
	70u16, 10u16, 4u16, 3u16, 21u16, 3u16, 12u16, 13u16, 4u16, 5u16, 2u16, 28u16, 1u16, 12u16,
	17u16, 7u16, 8u16, 32u16, 9u16, 38u16, 26u16, 1u16, 13u16, 12u16, 14u16, 14u16, 27u16, 14u16,
	52u16, 4u16, 25u16, 17u16, 26u16, 9u16, 15u16, 10u16, 2u16, 21u16, 8u16, 4u16, 1u16, 4u16,
	5u16, 6u16, 6u16, 3u16, 46u16, 9u16, 6u16, 4u16, 13u16, 29u16, 2u16, 8u16, 7u16, 1u16, 2u16,
	4u16, 1u16, 11u16, 6u16, 5u16, 9u16, 3u16, 14u16, 3u16, 5u16, 1u16, 13u16, 18u16, 10u16, 6u16,
	2u16, 20u16, 5u16, 3u16, 3u16, 15u16, 27u16, 18u16, 24u16, 3u16, 2u16, 5u16, 22u16, 1u16, 1u16,
	6u16, 7u16, 5u16, 4u16, 7u16, 11u16, 15u16, 5u16, 2u16, 5u16, 16u16, 7u16, 19u16, 1u16, 1u16,
	17u16, 4u16, 28u16, 1u16, 8u16, 23u16, 7u16, 6u16, 9u16, 8u16, 2u16, 2u16, 14u16, 25u16, 1u16,
	9u16, 26u16, 5u16, 1u16, 45u16, 5u16, 13u16, 2u16, 16u16, 40u16, 47u16, 1u16, 3u16, 44u16,
	12u16, 16u16, 1u16, 5u16, 5u16, 2u16, 11u16, 16u16, 9u16, 6u16, 1u16,
];
