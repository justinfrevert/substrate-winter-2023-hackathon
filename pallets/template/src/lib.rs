#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use sp_std::vec;
	use sp_std::vec::Vec;
	use core::fmt::Debug;
	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
	}

	#[pallet::error]
	pub enum Error<T> {
		/// This request was already claimed by a different compute provider
		AlreadyClaimed,
		/// No requests yet made
		NoRequests,
		/// The request could not be found, given the index
		RequestNotFound,
		/// Not allowed to request this proof
		UnauthorizedProofRequest
	}

	#[derive(
		Clone, Encode, Decode, Debug, Eq, PartialEq, MaxEncodedLen, TypeInfo, Default,
	)]
	pub enum ProofRequestStatus {
		#[default]
		Open,
		InProgress,
	}

	#[derive(
		Clone, Encode, Decode, Debug, Eq, PartialEq, MaxEncodedLen, TypeInfo, Default,
	)]
	// A request for a proof which verifies passage of some code's tests
	pub struct ProofRequest {
		/// Path of the resource
		full_name: BoundedVec<u8, ConstU32<1000>>,
		branch: Option<BoundedVec<u8, ConstU32<1000>>>,
		commit: Option<BoundedVec<u8, ConstU32<1000>>>,
		pub status: ProofRequestStatus
	}

	#[pallet::storage]
	pub(super) type ComputeProviders<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, bool>;

	#[pallet::storage]
	pub(super) type ProofRequesters<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, bool>;

	#[pallet::storage]
	pub(super) type ProofRequests<T: Config> = StorageValue<_, Vec<ProofRequest>>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(10000)]
		pub fn register_compute_provider(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ComputeProviders::<T>::insert(who, true);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10000)]
		pub fn register_requester(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ProofRequesters::<T>::insert(who, true);
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(10000)]
		pub fn request_proof(origin: OriginFor<T>, request: ProofRequest) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(ProofRequesters::<T>::contains_key(who), Error::<T>::UnauthorizedProofRequest);
			ProofRequests::<T>::append(request);
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(10000)]
		/// A claim by a compute provider for the rights to prove one request
		pub fn claim_request(origin: OriginFor<T>, request_index: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut requests = ProofRequests::<T>::get().ok_or(Error::<T>::NoRequests)?;

			ensure!(requests.len() > request_index as usize, Error::<T>::RequestNotFound);
			let mut proof_request = requests.get(request_index as usize).ok_or(Error::<T>::RequestNotFound)?.clone();

			ensure!(proof_request.status == ProofRequestStatus::Open, Error::<T>::AlreadyClaimed);
			proof_request.status = ProofRequestStatus::InProgress;

			requests[request_index as usize] = proof_request;

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(10000)]
		/// A compute provider fulfills a request by submitting a proof which fulfills the request, and any outputs given from the testing action
		pub fn fullfill_claimed_request(origin: OriginFor<T>, request_index: u32, proof: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Ok(())
		}
	}
}
