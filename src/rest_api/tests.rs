use super::{ApiState, config, server, validate_zones};
use crate::app::{AppSettings, ZoneSettings};
use actix_web::{test, web, App, http::StatusCode};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

struct ConfigFile {
    directory: PathBuf,
    path: PathBuf,
    contents: String,
}

impl ConfigFile {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("road-zones-{}", Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("conf.toml");
        let contents = include_str!("../../data/conf.toml").to_string();
        fs::write(&path, &contents).unwrap();
        Self { directory, path, contents }
    }

    fn state(&self) -> std::sync::Arc<ApiState> {
        let settings: AppSettings = toml::from_str(&self.contents).unwrap();
        let state = ApiState::new(&settings, self.path.to_str().unwrap()).unwrap();
        state.set_video_info(640, 480, 25.0);
        state
    }
}

impl Drop for ConfigFile {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn zone() -> ZoneSettings {
    ZoneSettings {
        id: "zone_test".to_string(),
        geometry: [[10, 10], [200, 10], [200, 200], [10, 200]],
        color_rgb: Some([255, 175, 243]),
    }
}

const COMMENTED_ZONES: &str = "\
# Settings above the zones
[input]
    video_src = 'camera.mp4'

# Zone notes
[[zones]] # Camera entrance
    # Stable identifier
    id  = 'zone_test' # Keep this note
    geometry = [
        [1_0, 10], # Point A
        [200, 10],
        [200, 200], # Point C
        [10, 200],
    ] # Image coordinates
    color_rgb = [255, 175, 243] # Pink
    custom_note = 'Keep unknown fields'

# Settings below the zones
[publishers.redis]
    host = 'localhost' # Redis host
";

#[actix_web::test]
async fn saving_unchanged_zones_preserves_the_document_exactly() {
    let updated = config::update_document(COMMENTED_ZONES, &Some(vec![zone()])).unwrap();
    assert_eq!(updated, COMMENTED_ZONES);
    let original = include_str!("../../data/conf.toml");
    let settings: AppSettings = toml::from_str(original).unwrap();
    assert_eq!(config::update_document(original, &settings.zones).unwrap(), original);
}

#[actix_web::test]
async fn editing_coordinates_keeps_comments_and_array_formatting() {
    let mut changed = zone();
    changed.geometry[2][0] = 190;
    changed.color_rgb = Some([255, 175, 240]);
    let updated = config::update_document(COMMENTED_ZONES, &Some(vec![changed.clone()])).unwrap();
    assert_eq!(updated, COMMENTED_ZONES.replace("[200, 200]", "[190, 200]").replace("[255, 175, 243]", "[255, 175, 240]"));
    let parsed: toml::Value = toml::from_str(&updated).unwrap();
    let saved: Vec<ZoneSettings> = parsed["zones"].clone().try_into().unwrap();
    assert_eq!(saved, vec![changed]);
}

#[actix_web::test]
async fn renaming_and_adding_zones_keeps_the_section_position_and_indent() {
    let mut renamed = zone();
    renamed.id = "entrance".to_string();
    let mut added = zone();
    added.id = "exit".to_string();
    let updated = config::update_document(COMMENTED_ZONES, &Some(vec![renamed, added])).unwrap();
    assert!(updated.contains("    id  = \"entrance\" # Keep this note"), "{updated}");
    assert!(updated.contains("\n[[zones]]\n    id = \"exit\"\n    geometry = "), "{updated}");
    assert_eq!(updated.matches("# Camera entrance").count(), 1);
    assert_eq!(updated.matches("# Zone notes").count(), 1);
    assert!(updated.rfind("[[zones]]").unwrap() < updated.find("[publishers.redis]").unwrap());
    assert!(updated.ends_with("# Settings below the zones\n[publishers.redis]\n    host = 'localhost' # Redis host\n"));
    toml::from_str::<toml::Value>(&updated).unwrap();
}

#[actix_web::test]
async fn deleting_and_reordering_zones_keeps_comments_with_their_ids() {
    let second = "\n# Exit notes\n[[zones]]\n    id = 'exit'\n    geometry = [[10, 10], [200, 10], [200, 200], [10, 200]]\n    color_rgb = [255, 175, 243]\n";
    let original = COMMENTED_ZONES.replace("\n# Settings below the zones", &format!("{}\n# Settings below the zones", second));
    let mut exit = zone();
    exit.id = "exit".to_string();
    let deleted = config::update_document(&original, &Some(vec![exit.clone()])).unwrap();
    assert!(deleted.contains(second), "{deleted}");
    assert!(!deleted.contains("# Camera entrance"));
    let reordered = config::update_document(&original, &Some(vec![exit, zone()])).unwrap();
    assert!(reordered.find("# Exit notes").unwrap() < reordered.find("# Zone notes").unwrap());
    assert_eq!(reordered.matches("# Camera entrance").count(), 1);
    let parsed: toml::Value = toml::from_str(&reordered).unwrap();
    assert_eq!(parsed["zones"][0]["id"].as_str(), Some("exit"));
    assert_eq!(parsed["zones"][1]["id"].as_str(), Some("zone_test"));
}

#[actix_web::test]
async fn clearing_and_restoring_optional_color_keeps_neighbouring_fields() {
    let mut uncolored = zone();
    uncolored.color_rgb = None;
    let without_color = config::update_document(COMMENTED_ZONES, &Some(vec![uncolored])).unwrap();
    assert_eq!(without_color, COMMENTED_ZONES.replace("    color_rgb = [255, 175, 243] # Pink\n", ""));
    let restored = config::update_document(&without_color, &Some(vec![zone()])).unwrap();
    assert!(restored.contains("    color_rgb = [255, 175, 243]\n"), "{restored}");
    assert!(restored.contains("    custom_note = 'Keep unknown fields'"));
}

#[actix_web::test]
async fn rejects_invalid_geometry_and_duplicate_ids() {
    let valid = zone();
    assert!(validate_zones(&[valid.clone()], 640, 480).is_ok());
    assert!(validate_zones(&[valid.clone(), valid.clone()], 640, 480).is_err());
    let mut invalid = valid.clone();
    invalid.geometry = [[10, 10], [200, 200], [200, 10], [10, 200]];
    assert!(validate_zones(&[invalid], 640, 480).is_err());
    let mut invalid = valid.clone();
    invalid.geometry[0] = [640, 10];
    assert!(validate_zones(&[invalid], 640, 480).is_err());
    let mut invalid = valid.clone();
    invalid.geometry[0] = [-1, 10];
    assert!(validate_zones(&[invalid], 640, 480).is_err());
    let mut invalid = valid;
    invalid.color_rgb = Some([256, 0, 0]);
    assert!(validate_zones(&[invalid], 640, 480).is_err());
}

#[actix_web::test]
async fn zone_serialization_preserves_other_settings_and_comments() {
    let original = include_str!("../../data/conf.toml");
    let original_value: toml::Value = toml::from_str(original).unwrap();
    for zones in [None, Some(vec![]), Some(vec![zone()])] {
        let updated = config::update_document(original, &zones).unwrap();
        let settings: AppSettings = toml::from_str(&updated).unwrap();
        assert_eq!(settings.zones, zones);
        let value: toml::Value = toml::from_str(&updated).unwrap();
        for key in ["application_info", "input", "output", "detection", "tracking", "publishers", "rest_api"] {
            assert_eq!(value.get(key), original_value.get(key));
        }
        assert!(updated.contains("# Just field for future identification of application."));
        assert!(updated.contains("# Target classes to be used in filtering."));
    }
}

#[actix_web::test]
async fn runtime_changes_require_revision_and_separate_persistence() {
    let file = ConfigFile::new();
    let state = file.state();
    let app = test::init_service(App::new().app_data(web::Data::from(state.clone())).configure(server::routes)).await;
    let response = test::call_service(&app, test::TestRequest::post().uri("/api/zones").set_json(json!({
        "revision": 0, "zones": [zone()], "whole_frame": false
    })).to_request()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(fs::read_to_string(&file.path).unwrap(), file.contents);
    assert_eq!(state.zones_since(0).unwrap().0, 1);
    let response = test::call_service(&app, test::TestRequest::post().uri("/api/zones").set_json(json!({
        "revision": 0, "zones": [], "whole_frame": true
    })).to_request()).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let response = test::call_service(&app, test::TestRequest::post().uri("/api/mutations/save_config").set_json(json!({"revision": 1})).to_request()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let saved: Value = test::read_body_json(response).await;
    let backup = file.directory.join(saved["backup"].as_str().unwrap());
    assert_eq!(fs::read_to_string(backup).unwrap(), file.contents);
    let contents = fs::read_to_string(&file.path).unwrap();
    let settings: AppSettings = toml::from_str(&contents).unwrap();
    assert_eq!(settings.zones, Some(vec![zone()]));
    assert_eq!(state.zones.read().unwrap().saved_revision, 1);
}

#[actix_web::test]
async fn refuses_to_overwrite_zones_edited_in_the_file() {
    let file = ConfigFile::new();
    let state = file.state();
    let external = config::update_document(&file.contents, &Some(vec![zone()])).unwrap();
    fs::write(&file.path, &external).unwrap();
    let app = test::init_service(App::new().app_data(web::Data::from(state)).configure(server::routes)).await;
    let response = test::call_service(&app, test::TestRequest::post().uri("/api/mutations/save_config").set_json(json!({"revision": 0})).to_request()).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(fs::read_to_string(&file.path).unwrap(), external);
}

#[actix_web::test]
async fn crud_and_invalid_changes_leave_valid_state() {
    let file = ConfigFile::new();
    let state = file.state();
    let app = test::init_service(App::new().app_data(web::Data::from(state.clone())).configure(server::routes)).await;
    let response = test::call_service(&app, test::TestRequest::post().uri("/api/mutations/zones/create").set_json(json!({"revision": 0, "zone": zone()})).to_request()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut renamed = zone();
    renamed.id = "renamed".to_string();
    let response = test::call_service(&app, test::TestRequest::post().uri("/api/mutations/zones/update").set_json(json!({"revision": 1, "zone_id": "zone_test", "zone": renamed})).to_request()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = test::call_service(&app, test::TestRequest::post().uri("/api/mutations/zones/delete").set_json(json!({"revision": 2, "zone_id": "renamed"})).to_request()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut invalid = zone();
    invalid.geometry = [[0, 0]; 4];
    let response = test::call_service(&app, test::TestRequest::post().uri("/api/zones").set_json(json!({"revision": 3, "zones": [invalid]})).to_request()).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(state.zones.read().unwrap().revision, 3);
}
