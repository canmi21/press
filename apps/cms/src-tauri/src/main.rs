#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn overview_snapshot() -> Result<cms::overview::Snapshot, String> {
	cms::overview::snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
fn article_listing() -> Result<cms::articles::Listing, String> {
	cms::articles::listing().map_err(|error| error.to_string())
}

#[tauri::command]
fn derived_report() -> Result<cms::derived::Report, String> {
	cms::derived::report().map_err(|error| error.to_string())
}

#[tauri::command]
fn start_favicon_collection() -> Result<u32, String> {
	let repository = cms::paths::repo_root().map_err(|error| error.to_string())?;
	if let Some(run) =
		cms::task::registry::running(&repository, "favicon").map_err(|error| error.to_string())?
	{
		return Ok(run.pid);
	}

	let pid = std::process::id();
	std::thread::Builder::new()
		.name("favicon-collection".to_owned())
		.spawn(move || {
			if let Err(error) = cms::favicon::collect::from_articles(
				&repository,
				false,
				cms::task::registry::Shell::Desktop,
				Box::new(cms::task::progress::Silent),
			) {
				eprintln!("favicon collection failed: {error}");
			}
		})
		.map_err(|error| error.to_string())?;
	Ok(pid)
}

/// What the named articles are carrying, or the whole corpus when the list is empty.
#[tauri::command]
fn segment_sweep(articles: Vec<String>) -> Result<cms::gc::segments::Sweep, String> {
	let repository = cms::paths::repo_root().map_err(|error| error.to_string())?;
	cms::gc::segments::plan(&repository.join("contents"), &articles)
		.map_err(|error| error.to_string())
}

/// Drop them, and report how many entries went.
///
/// Synchronous, and not a task. The catalogue is for work that takes minutes, asks a model or
/// cannot be run twice at once; this is a YAML rewrite per article, taken under the record's own
/// lock. Giving it a progress bar would be describing something nobody can watch.
#[tauri::command]
fn sweep_segments(articles: Vec<String>) -> Result<usize, String> {
	let repository = cms::paths::repo_root().map_err(|error| error.to_string())?;
	let contents = repository.join("contents");
	let sweep =
		cms::gc::segments::plan(&contents, &articles).map_err(|error| error.to_string())?;
	cms::gc::segments::apply(&repository, &contents, &sweep).map_err(|error| error.to_string())
}

#[tauri::command]
fn live_task_runs() -> Result<Vec<cms::task::registry::Run>, String> {
	let repository = cms::paths::repo_root().map_err(|error| error.to_string())?;
	// Poll the machine-wide registry instead of subscribing to Tauri events: events could only
	// expose runs started by this window, while the registry also includes terminal commands.
	cms::task::registry::live(&repository).map_err(|error| error.to_string())
}

fn main() {
	let builder = tauri::Builder::default();
	#[cfg(desktop)]
	let builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());
	#[cfg(all(desktop, debug_assertions, feature = "mcp"))]
	let builder =
		builder.plugin(tauri_plugin_mcp_bridge::Builder::new().bind_address("127.0.0.1").build());

	builder
		.invoke_handler(tauri::generate_handler![
			overview_snapshot,
			article_listing,
			derived_report,
			start_favicon_collection,
			segment_sweep,
			sweep_segments,
			live_task_runs
		])
		.run(tauri::generate_context!())
		.expect("could not run the CMS desktop client");
}
