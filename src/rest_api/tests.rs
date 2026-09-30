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
