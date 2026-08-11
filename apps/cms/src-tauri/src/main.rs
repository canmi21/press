#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn overview_snapshot() -> Result<cms::overview::Snapshot, String> {
	cms::overview::snapshot().map_err(|error| error.to_string())
}

fn main() {
	let builder = tauri::Builder::default();
	#[cfg(desktop)]
	let builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());
	#[cfg(all(desktop, debug_assertions, feature = "mcp"))]
	let builder = builder.plugin(
		tauri_plugin_mcp_bridge::Builder::new()
			.bind_address("127.0.0.1")
			.build(),
	);

	builder
		.invoke_handler(tauri::generate_handler![overview_snapshot])
		.run(tauri::generate_context!())
		.expect("could not run the CMS desktop client");
}
