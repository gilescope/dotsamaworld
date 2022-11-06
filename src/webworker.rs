use crate::{
	datasource, do_datasources, log, render_block, BridgeMessage, RenderUpdate, BASETIME,
	DATASOURCE_EPOC, UPDATE_QUEUE,
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

	async fn send_it_too(render_block: RenderUpdate) {
		let mut pending = UPDATE_QUEUE.lock().unwrap();
		pending.extend(render_block);
	}
}

impl Worker for IOWorker {
	type Input = BridgeMessage;
	type Message = Vec<()>;
	type Output = RenderUpdate;

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
				async_std::task::block_on(do_datasources(sovs, as_of, &Self::send_it_too));
			},
			BridgeMessage::GetNewBlocks => {
				let vec = &mut *UPDATE_QUEUE.lock().unwrap();
				let mut results = RenderUpdate::default();
				core::mem::swap(vec, &mut results);
				scope.respond(id, results);
			},
		}

		// 	let chain_info = ChainInfo{
		// 		chain_ws: String::from("kusama-rpc.polkadot.io"),
		// // pub chain_id: Option<NonZeroU32>,
		// // pub chain_drawn: bool,
		// // Negative is other direction from center.
		// 		chain_index: 1,
		// 		chain_url: DotUrl{ sovereign:Some(1), env:Env::Prod, ..DotUrl::default() },
		// 	};
		// 	// let url = chain_name_to_url(&chain_info.chain_ws);
		// 	// let source = datasource::RawDataSource::new(&url);
		// 	let block_watcher = datasource::BlockWatcher{
		// 				tx: None,
		// 				chain_info ,
		// 				as_of: None,
		// 				receive_channel: None,
		// 				sender: None,
		// 			};

		// 	async_std::task::block_on(block_watcher.watch_blocks());
		// self.link.respond(id, (msg, 42));
	}
}
