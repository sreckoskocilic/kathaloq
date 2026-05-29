#[tauri::command]
pub fn third_party_licenses() -> &'static str {
    include_str!("../../THIRD-PARTY-LICENSES.html")
}
