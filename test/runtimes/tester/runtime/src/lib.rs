#![cfg_attr(not(feature = "std"), no_std)]

// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit="256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use sp_std::prelude::*;
use sp_core::{Decode, crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	generic, create_runtime_str, print, impl_opaque_keys,
	ApplyExtrinsicResult, MultiSignature,
	transaction_validity::{TransactionValidity, TransactionSource},
};
use sp_runtime::traits::{
	BlakeTwo256, Block as BlockT, IdentityLookup, Verify,
	IdentifyAccount, NumberFor, Saturating,
};
use sp_api::impl_runtime_apis;
use babe::{AuthorityId as BabeId};
use grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use balances::Call as BalancesCall;
pub use sp_runtime::{Permill, Perbill};
pub use frame_support::{
	construct_runtime, parameter_types, storage_root, runtime_print,
	traits::{KeyOwnerProofSystem, Randomness},
	weights::{Weight, RuntimeDbWeight, constants::WEIGHT_PER_SECOND},
};

/// An index to a block.
pub type BlockNumber = u32;

/// A timestamp or duration.
pub type Moment = u64;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub babe: Babe,
			pub grandpa: Grandpa,
		}
	}
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("polkadot"),
	impl_name: create_runtime_str!("host-tester"),
	authoring_version: 2,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

/// Targeted block time.
pub const MILLISECS_PER_BLOCK: Moment = 6000;

/// Babe slot duration.
pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// Babe epoch duration.
pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 4 * HOURS;

/// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;

	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub const MaximumBlockWeight: Weight = 2 * WEIGHT_PER_SECOND;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);

	/// Assume 10% of weight for average on_initialize calls.
	pub MaximumExtrinsicWeight: Weight = AvailableBlockRatio::get()
		.saturating_sub(Perbill::from_percent(10)) * MaximumBlockWeight::get();

	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;

	pub const DbWeight: RuntimeDbWeight = RuntimeDbWeight {
		read: 60_000_000,
		write: 200_000_000,
	};

	pub const ExtrinsicBaseWeight: Weight = 100_000_000;
	pub const BlockExecutionWeight: Weight = 1_000_000_000;

	pub const Version: RuntimeVersion = VERSION;
}

// Configure FRAME pallets to include in runtime.
impl system::Trait for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = ();
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<AccountId>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Maximum weight of each block.
	type MaximumBlockWeight = MaximumBlockWeight;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = DbWeight;
	/// The weight of the overhead invoked on the block import process, independent of the
	/// extrinsics included in that block.
	type BlockExecutionWeight = BlockExecutionWeight;
	/// The base weight of any extrinsic processed by the runtime, independent of the
	/// logic of that extrinsic. (Signature verification, nonce increment, fee, etc...)
	type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
	/// The maximum weight that a single extrinsic of `Normal` dispatch class can have,
	/// idependent of the logic of that extrinsics. (Roughly max block weight - average on
	/// initialize cost).
	type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
	/// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
	type MaximumBlockLength = MaximumBlockLength;
	/// Portion of the block weight that is available to all normal transactions.
	type AvailableBlockRatio = AvailableBlockRatio;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl timestamp::Trait for Runtime {
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_BLOCKS as u64;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}

impl babe::Trait for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = babe::SameAuthoritiesForever; // TODO: Check authority change

	type KeyOwnerProofSystem = (); // Disabled

	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, BabeId)>>::Proof;

	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		BabeId,
	)>>::IdentificationTuple;

	type HandleEquivocation = (); // Disabled

	type WeightInfo = ();
}

impl grandpa::Trait for Runtime {
	type Event = Event;

	type Call = Call;

	type KeyOwnerProofSystem = (); // Disabled

	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;

	type HandleEquivocation = (); // Disabled

	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const MaxLocks: u32 = 50;
}

impl balances::Trait for Runtime {
	type MaxLocks = MaxLocks;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

impl sudo::Trait for Runtime {
	type Event = Event;
	type Call = Call;
}

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: system::{Module, Call, Config, Storage, Event<T>},
		CollectiveFlip: collective_flip::{Module, Call, Storage},
		Babe: babe::{Module, Call, Storage, Config, Inherent},
		Grandpa: grandpa::{Module, Call, Storage, Config, Event},
		Timestamp: timestamp::{Module, Call, Storage, Inherent},
		Balances: balances::{Module, Call, Storage, Config<T>, Event<T>},
		Sudo: sudo::{Module, Call, Config<T>, Storage, Event<T>},
	}
);

/// The address format for describing accounts.
pub type Address = AccountId;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	system::CheckSpecVersion<Runtime>,
	system::CheckTxVersion<Runtime>,
	system::CheckGenesis<Runtime>,
	system::CheckEra<Runtime>,
	system::CheckNonce<Runtime>,
	system::CheckWeight<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<Runtime, Block, system::ChainContext<Runtime>, Runtime, AllModules>;


/// Print current storage root
fn print_storage_root() {
	let storage_root = Hash::decode(&mut &storage_root()[..])
		.expect("`storage_root` is a valid hash");
	runtime_print!("##{:x}##", storage_root);
}


impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			print("@@version()@@");
			print_storage_root();
			VERSION
		}

		fn execute_block(block: Block) {
			print("@@execute_block()@@");
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			print("@@initialize_block()@@");
			Executive::initialize_block(header)
		}
	} // core api

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			print("@@metadata()@@");
			Runtime::metadata().into()
		}
	} // metadata api

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			print("@@apply_extrinsics()@@");
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			print("@@finalize_block()@@");
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			print("@@inherent_extrinsics()@@");
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			print("@@check_inherents()@@");
			data.check_extrinsics(&block)
		}

		fn random_seed() -> <Block as BlockT>::Hash {
			print("@@random_seed()@@");
			CollectiveFlip::random_seed()
		}
	} // block builder

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic
		) -> TransactionValidity {
			print("@@validate_transaction()@@");
			Executive::validate_transaction(source, tx)
		}
	} // transaction pool

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			print("@@generate_session_keys()@@");
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			print("@@decode_session_keys()@@");
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	} // session

	impl sp_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_babe::BabeGenesisConfiguration {
			print("@@configuration()@@");
			print_storage_root();
			sp_babe::BabeGenesisConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: PRIMARY_PROBABILITY,
				genesis_authorities: Babe::authorities(),
				randomness: Babe::randomness(),
				allowed_slots: sp_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
			}
		}

		fn current_epoch_start() -> sp_babe::SlotNumber {
			print("@@current_epoch_start()@@");
			Babe::current_epoch_start()
		}


		fn generate_key_ownership_proof(
			_slot_number: sp_babe::SlotNumber,
			_authority_id: sp_babe::AuthorityId,
		) -> Option<sp_babe::OpaqueKeyOwnershipProof> {
			None
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_babe::EquivocationProof<<Block as BlockT>::Header>,
			_key_owner_proof: sp_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}
	} // babe

	impl sp_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			print("@@grandpa_authorities()@@");
			Grandpa::grandpa_authorities()
		}

		fn generate_key_ownership_proof(
			_set_id: sp_grandpa::SetId,
			_authority_id: GrandpaId,
		) -> Option<sp_grandpa::OpaqueKeyOwnershipProof> {
			None
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: sp_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}
	} // grandpa
}
