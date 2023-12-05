#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;


#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		inherent::Vec,
		pallet_prelude::*,
		traits::{BalanceStatus, Currency, ReservableCurrency},
	};
	use frame_system::pallet_prelude::*;
	use risc0_zkvm::{SegmentReceipt, SessionReceipt};

	type ImageId = [u32; 8];

	#[pallet::pallet]
	// TODO: Needs proper BoundedVec encoding from offchain in order to get bounded types working
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Currency: Currency<<Self as frame_system::Config>::AccountId>
			+ ReservableCurrency<Self::AccountId>;
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type MaxArgsLength: Get<u32>;
		// Max length of programs
		type MaxProgramLength: Get<u32>;
		// Max Length of proofs
		type MaxProofLength: Get<u32>;
	}

	#[derive(
		Clone, Encode, Decode, Debug, Eq, PartialEq, MaxEncodedLen, TypeInfo, Default,
	)]
	pub enum ProofRequestStatus {
		#[default]
		Open,
		InProgress,
	}

	#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, TypeInfo)]
	/// Sufficient details for some party to retrieve source code information. 
	/// NOTE: only public git repositories are supported at this time
	pub struct TestProofDetails {
		full_name: Vec<u8>,
		branch: Option<BoundedVec<u8, ConstU32<1000>>>,
		commit: Option<BoundedVec<u8, ConstU32<1000>>>,
		pub status: ProofRequestStatus
	}

	#[pallet::storage]
	pub(super) type TestComputeProviders<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, bool>;

	#[pallet::storage]
	pub(super) type TestProofRequesters<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, bool>;


	#[pallet::storage]
	type TestProofRequests<T: Config> = StorageValue<
		_,
		Vec<TestProofDetails>,
		ValueQuery
	>;

	#[pallet::storage]
	pub(super) type TestProofs<T: Config> =
		StorageMap<_, Blake2_128Concat, BoundedVec<u8, ConstU32<1000>>, Vec<(Vec<u32>, u32)>, OptionQuery>;
	

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ProofRequested {
			image_id: ImageId,
			args: Vec<Vec<u32>>,
		},
		/// Proof was successfully verified and will be stored
		ProofVerified,
		/// A program was uploaded
		ProgramUploaded {
			image_id: ImageId,
		},
		TestProofVerified(TestProofDetails)
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Another compute provider already claimed the request
		AlreadyClaimed,
		/// No test requests available
		NoRequests,
		RequestNotFound,
		/// Tried to upload a program which already exists
		ProgramAlreadyExists,
		/// Tried to verify a proof but the program did not exist
		ProgramDoesNotExist,
		/// Could not verify proof
		ProofInvalid,
		/// Proof did not pass verification
		ProofNotVerified,
		/// This test proof was not found
		TestProofNotFound,
		/// This account is not authorized to perform the requested action
		UnauthorizedProofRequest
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(10000)]
		pub fn register_compute_provider(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			TestComputeProviders::<T>::insert(who, true);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10000)]
		pub fn register_requester(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			TestProofRequesters::<T>::insert(who, true);
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(10000)]
		/// A claim by a compute provider for the rights to prove one request
		pub fn claim_request(origin: OriginFor<T>, request_index: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut requests = TestProofRequests::<T>::get();

			ensure!(requests.len() > request_index as usize, Error::<T>::RequestNotFound);
			let mut proof_request = requests.get(request_index as usize).ok_or(Error::<T>::RequestNotFound)?.clone();

			ensure!(proof_request.status == ProofRequestStatus::Open, Error::<T>::AlreadyClaimed);
			proof_request.status = ProofRequestStatus::InProgress;

			requests[request_index as usize] = proof_request;

			Ok(())
		}

		/// Request proof of some test execution
		#[pallet::call_index(5)]
		#[pallet::weight(100000)]
		pub fn request_test_proof(
			origin: OriginFor<T>,
			test_proof_details: TestProofDetails,
			reward: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(TestProofRequesters::<T>::contains_key(&who), Error::<T>::UnauthorizedProofRequest);

			T::Currency::reserve(&who, reward)?;

			TestProofRequests::<T>::append(test_proof_details);
			// Self::deposit_event(Event::ProofRequested { image_id, args });

			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(100000)]
		pub fn store_and_verify_test_proof(
			origin: OriginFor<T>,
			image_id: ImageId,
			receipt_data: Vec<(Vec<u32>, u32)>,
			journal: Vec<u8>,
			// Index of the index of the stored test proof which we are verifying
			test_proof_details_index: u32
		) -> DispatchResult {
			ensure_signed(origin)?;

			let mut test_proof_requests = TestProofRequests::<T>::get();
			let test_proof_request = test_proof_requests
				.get(test_proof_details_index as usize)
				.ok_or(Error::<T>::TestProofNotFound)?
				.clone();

			test_proof_requests.remove(test_proof_details_index as usize);

			let segments: Vec<SegmentReceipt> = receipt_data
				.clone()
				.into_iter()
				.map(|(seal, index)| SegmentReceipt { seal, index })
				.collect();
			let receipt = SessionReceipt { segments, journal };
			receipt.verify(image_id).map_err(|_| Error::<T>::ProofNotVerified)?;
			

			let commit = test_proof_request.clone().commit.expect("Commit should exist");
			TestProofs::<T>::insert(commit, receipt_data);
			Self::deposit_event(Event::<T>::TestProofVerified(test_proof_request));
			Ok(())
		}
	}
}
