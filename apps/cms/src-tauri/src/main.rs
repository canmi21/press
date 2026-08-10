#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
		.run(tauri::generate_context!())
		.expect("could not run the CMS desktop client");
}
