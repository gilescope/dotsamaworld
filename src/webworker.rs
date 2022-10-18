use crate::{
	datasource, do_datasources, log, BridgeMessage, BASETIME, DATASOURCE_EPOC, UPDATE_QUEUE,
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

	async fn send_it_too(blocks: Vec<datasource::DataUpdate>) {
		// web_sys::console::log_1(&format!("got block. add to worker queue{}",
		// blocks.len()).into());

		// Could move this earlier to when a block is produced by relay chain?
		let mut base_time = *BASETIME.lock().unwrap();
		if base_time == 0 {
			if let datasource::DataUpdate::NewBlock(block) = &blocks[0] {
				base_time = block.timestamp.unwrap_or(0);
				web_sys::console::log_1(&format!("BASETIME set to {}", base_time).into());
				*BASETIME.lock().unwrap() = base_time;
			}
		}

		UPDATE_QUEUE.lock().unwrap().extend(blocks);
		// web_sys::console::log_1(&format!("added to worker queue").into());
	}
}

impl Worker for IOWorker {
	type Input = BridgeMessage;
	type Message = Vec<()>;
	type Output = Vec<datasource::DataUpdate>;

	fn create(_scope: &WorkerScope<Self>) -> Self {
		Self {}
	}

	fn update(&mut self, _scope: &WorkerScope<Self>, msg: Self::Message) {
		async_std::task::block_on(Self::async_update(msg));
	}

	fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
		match msg {
			BridgeMessage::SetDatasource(s, as_of, data_epoc) => {
				DATASOURCE_EPOC.store(data_epoc, Ordering::Relaxed);
				// web_sys::console::log_1(&format!("got input from bridge basetime {}",
				// basetime).into()); let link_clone : Arc<async_std::sync::Mutex<WorkerLink<Self>>>
				// = scope.clone();
				async_std::task::block_on(do_datasources(s, as_of, &Self::send_it_too));
				// 			async |_|{
				// 			web_sys::console::log_1(&format!("got block. send to bridge").into());
				// 			self.t();
				// //			scope.send_message(vec![]);
				// 		}
			},
			BridgeMessage::GetNewBlocks => {
				// let t = async move || {
				let vec = &mut *UPDATE_QUEUE.lock().unwrap();
				let mut results = vec![];
				core::mem::swap(vec, &mut results);
				scope.respond(id, results);
				// };
				// async_std::task::block_on(t());
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
