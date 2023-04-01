use crate::{
	do_datasources, log, BridgeMessage, ChainInfo, Details, RenderDetails, RenderUpdate,
	DATASOURCE_EPOC, DETAILS, SOVEREIGNS, UPDATE_QUEUE,
};
use core::sync::atomic::Ordering;

macro_rules! log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

use core::time::Duration;
use gloo_worker::WorkerScope;

use gloo_worker::{HandlerId, Worker};

pub struct IOWorker {}

impl IOWorker {
	pub async fn async_update(_msg: <Self as Worker>::Message) {
		log!("Got update");
		async_std::task::sleep(Duration::from_secs(5)).await;
		async_std::task::sleep(Duration::from_secs(5)).await;
		log!("Finished waiting");
	}

	async fn send_it_too(rendered: (RenderUpdate, RenderDetails)) {
		let mut pending = UPDATE_QUEUE.lock().unwrap();
		let mut details = DETAILS.lock().unwrap();
		pending.extend(rendered.0);
		details.extend(rendered.1);
	}
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum WorkerResponse {
	RenderUpdate(RenderUpdate, u64), //free transactions
	Details(Vec<(u32, Details, ChainInfo)>),
}

impl Worker for IOWorker {
	type Input = BridgeMessage;
	type Message = Vec<()>;
	type Output = WorkerResponse;

	fn create(_scope: &WorkerScope<Self>) -> Self {
		Self {}
	}

	fn update(&mut self, _scope: &WorkerScope<Self>, msg: Self::Message) {
		async_std::task::block_on(Self::async_update(msg));
	}

	fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
		match msg {
			BridgeMessage::SetDatasource(sovs, as_of, data_epoc) => {
				DATASOURCE_EPOC.store(data_epoc, Ordering::Relaxed);
				*SOVEREIGNS.lock().unwrap() = Some(sovs.clone());
				async_std::task::block_on(do_datasources(sovs, as_of, &Self::send_it_too));
			},
			BridgeMessage::GetNewBlocks => {
				let vec = &mut *UPDATE_QUEUE.lock().unwrap();
				// If a chain does not have any transactions then assume the average.
				let chains = crate::CHAIN_STATS.lock().unwrap();
				let chain_count = chains.values().count() as u64;
				if chain_count > 0 {
					let chains_with_no_tx = chains
						.values()
						.map(|v| v.avg_free_transactions())
						.filter_map(|s| if s.is_none() { Some(()) } else { None })
						.count() as u64;
					// log!("chains with no tx: {}", chains_with_no_tx);
					let mut free_tx = chains
						.values()
						.map(|v| v.avg_free_transactions())
						.filter_map(|s| s)
						.sum::<u64>() / 12;
					//TODO 12 seconds per block assumed.

					//For chains with no transactions assume average
					free_tx += free_tx * chains_with_no_tx / chain_count as u64;

					let mut results = RenderUpdate::default();
					core::mem::swap(vec, &mut results);
					if results.any() {
						scope.respond(id, WorkerResponse::RenderUpdate(results, free_tx));
					}
				}
			},
			BridgeMessage::GetEventDetails(cube_index) => {
				let instances = &DETAILS.lock().unwrap().event_instances;
				let details = instances[cube_index as usize].clone();
				let sov_lock = SOVEREIGNS.lock().unwrap();
				let sovs = sov_lock.as_ref().unwrap();
				let chain_info = sovs.chain_info(&details.doturl);
				log!("respond to selected item request");
								
				let links = details.links.clone();
				let mut results = vec![(cube_index, details, chain_info)];

				for link in links {
					let details = instances[cube_index as usize].clone();
					let chain_info = sovs.chain_info(&details.doturl);
					results.push((link as u32, details, chain_info));
				}

				scope.respond(id, WorkerResponse::Details(results));
			},
			BridgeMessage::GetExtrinsicDetails(cube_index) => {
				let details =
					DETAILS.lock().unwrap().extrinsic_instances[cube_index as usize].clone();
				let chain_info =
					(*SOVEREIGNS.lock().unwrap()).as_ref().unwrap().chain_info(&details.doturl);
				scope.respond(id, WorkerResponse::Details(vec![(cube_index, details, chain_info)]));
			},
		}
	}
}
