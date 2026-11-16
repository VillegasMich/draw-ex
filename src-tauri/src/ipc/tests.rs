//! Every command through Tauri's mock runtime, as the webview would call it.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tauri::http::{HeaderMap, HeaderValue};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
};
use tauri::webview::InvokeRequest;
use tauri::{App, Listener, WebviewWindow, WebviewWindowBuilder};

use super::frames::{
    CAMERA_ID_HEADER, FRAME_HEIGHT_HEADER, FRAME_TIME_MS_HEADER, FRAME_WIDTH_HEADER,
};

struct TestApp {
    app: App<MockRuntime>,
    webview: WebviewWindow<MockRuntime>,
}

impl TestApp {
    fn new() -> Self {
        let app = super::register_commands(mock_builder())
            .build(mock_context(noop_assets()))
            .expect("the mock app builds");
        super::mount_events(&app);
        let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the mock webview builds");
        Self { app, webview }
    }

    fn invoke_raw(&self, cmd: &str, body: InvokeBody, headers: HeaderMap) -> Result<Value, Value> {
        let url = if cfg!(windows) {
            "http://tauri.localhost"
        } else {
            "tauri://localhost"
        };
        get_ipc_response(
            &self.webview,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: url.parse().expect("valid URL"),
                body,
                headers,
                invoke_key: INVOKE_KEY.into(),
            },
        )
        .map(|body| body.deserialize().expect("the response is JSON"))
    }

    fn invoke(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        self.invoke_raw(cmd, InvokeBody::Json(args), HeaderMap::new())
    }

    /// Collects the JSON payloads of every `event` emitted from now on.
    fn record(&self, event: &str) -> Arc<Mutex<Vec<Value>>> {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        self.app.listen_any(event, move |event| {
            let payload = serde_json::from_str(event.payload()).expect("payload is JSON");
            sink.lock().expect("not poisoned").push(payload);
        });
        seen
    }
}

#[test]
fn set_camera_setup_accepts_every_setup() {
    let app = TestApp::new();
    let setups = [
        json!({ "kind": "oneSide", "camera": "cam-1", "player": "a" }),
        json!({
            "kind": "splitLine",
            "camera": "cam-1",
            "line": { "from": { "x": 640.0, "y": 360.0 }, "to": { "x": 0.0, "y": 360.0 } },
        }),
        json!({ "kind": "twoCameras", "a": "cam-1", "b": "cam-2" }),
    ];
    for setup in setups {
        assert_eq!(
            app.invoke("set_camera_setup", json!({ "setup": setup })),
            Ok(Value::Null)
        );
    }
}

#[test]
fn start_session_returns_a_session_with_a_wall_clock_start() {
    assert_eq!(
        TestApp::new().invoke("start_session", json!({})),
        Ok(json!({ "id": "canned-session", "startedAtMs": 1_763_135_520_000.0 }))
    );
}

#[test]
fn end_session_returns_the_session_summary() {
    assert_eq!(
        TestApp::new().invoke("end_session", json!({})),
        Ok(json!({
            "id": "canned-session",
            "confirmedCount": 3,
            "logPath": "duels/canned-session.jsonl",
        }))
    );
}

#[test]
fn get_card_returns_the_canned_cards_and_null_for_unknown_passcodes() {
    let app = TestApp::new();

    let reborn = app
        .invoke(
            "get_card",
            json!({ "passcode": 83_764_719, "language": "en" }),
        )
        .expect("get_card succeeds");
    assert_eq!(
        reborn,
        json!({
            "passcode": 83_764_719,
            "name": "Monster Reborn",
            "kind": "Spell Card",
            "description": "Target 1 monster in either GY; Special Summon it.",
            "attribute": null,
            "race": "Normal",
            "level": null,
            "linkRating": null,
            "atk": null,
            "def": null,
        })
    );

    let ash = app
        .invoke(
            "get_card",
            json!({ "passcode": 14_558_127, "language": "fr" }),
        )
        .expect("get_card succeeds");
    assert_eq!(ash["name"], "Ash Blossom & Joyous Spring");
    assert_eq!(ash["level"], 3);
    assert_eq!(ash["def"], 1800);

    let pot = app
        .invoke(
            "get_card",
            json!({ "passcode": 55_144_522, "language": "en" }),
        )
        .expect("get_card succeeds");
    assert_eq!(pot["name"], "Pot of Greed");

    assert_eq!(
        app.invoke("get_card", json!({ "passcode": 1, "language": "en" })),
        Ok(Value::Null)
    );
}

#[test]
fn deck_commands_return_the_canned_deck() {
    let app = TestApp::new();
    let deck = json!({ "id": "canned-deck", "name": "Canned Deck", "cardCount": 40 });

    assert_eq!(app.invoke("list_decks", json!({})), Ok(json!([deck])));
    assert_eq!(
        app.invoke("import_deck", json!({ "path": "/home/me/deck.ydk" })),
        Ok(deck)
    );
    assert_eq!(
        app.invoke("select_deck", json!({ "id": "canned-deck" })),
        Ok(Value::Null)
    );
    assert_eq!(
        app.invoke("select_deck", json!({ "id": null })),
        Ok(Value::Null)
    );
}

#[test]
fn get_settings_returns_the_defaults_and_update_settings_echoes() {
    let app = TestApp::new();
    assert_eq!(
        app.invoke("get_settings", json!({})),
        Ok(json!({
            "language": "en",
            "modelVariant": "fp16",
            "inferenceFps": 6,
            "selectedDeck": null,
            "imageCacheMb": 1024,
        }))
    );

    let changed = json!({
        "language": "de",
        "modelVariant": "int8",
        "inferenceFps": 10,
        "selectedDeck": "canned-deck",
        "imageCacheMb": 256,
    });
    assert_eq!(
        app.invoke("update_settings", json!({ "settings": changed })),
        Ok(changed)
    );
}

#[test]
fn model_status_lists_every_variant_with_fp16_installed() {
    let status = TestApp::new()
        .invoke("model_status", json!({}))
        .expect("model_status succeeds");

    assert_eq!(status["current"], "fp16");
    let variants = status["variants"].as_array().expect("variants is an array");
    let names: Vec<&str> = variants
        .iter()
        .map(|v| v["variant"].as_str().expect("variant is a string"))
        .collect();
    assert_eq!(names, ["fp32", "fp16", "int8", "yugiscanInt8"]);
    for variant in variants {
        let installed = variant["variant"] == "fp16";
        assert_eq!(variant["installed"], installed);
        let total = variant["bytesTotal"]
            .as_u64()
            .expect("bytesTotal is a number");
        assert!(total > 0, "{variant}");
        assert_eq!(variant["bytesOnDisk"], if installed { total } else { 0 });
    }
}

#[test]
fn download_models_emits_three_progress_events_for_the_variants_files() {
    let app = TestApp::new();
    let events = app.record("model-download-progress");

    assert_eq!(
        app.invoke("download_models", json!({ "variant": "int8" })),
        Ok(Value::Null)
    );

    let events = events.lock().expect("not poisoned");
    let files: Vec<&str> = events
        .iter()
        .map(|e| e["file"].as_str().expect("file is a string"))
        .collect();
    assert_eq!(
        files,
        [
            "onnx/ygo_yolo.onnx",
            "onnx/vit_int8.onnx",
            "onnx/cardnames_onnx.json"
        ]
    );
    for event in events.iter() {
        assert_eq!(event["variant"], "int8");
        assert_eq!(event["bytesDone"], event["bytesTotal"]);
    }
    assert_eq!(events[1]["bytesTotal"], 97_771_550);
}

#[test]
fn download_models_emits_three_progress_events_for_every_variant() {
    for variant in ["fp32", "fp16", "int8", "yugiscanInt8"] {
        let app = TestApp::new();
        let events = app.record("model-download-progress");

        assert_eq!(
            app.invoke("download_models", json!({ "variant": variant })),
            Ok(Value::Null)
        );

        assert_eq!(events.lock().expect("not poisoned").len(), 3, "{variant}");
    }
}

#[test]
fn cache_commands_return_stats_and_clear_every_kind() {
    let app = TestApp::new();
    let stats = app
        .invoke("cache_stats", json!({}))
        .expect("cache_stats succeeds");
    assert_eq!(stats["imageCount"], 3);
    assert_eq!(stats["cardDbVersion"], "canned");
    for key in ["imagesBytes", "cardDbBytes", "modelsBytes"] {
        assert!(stats[key].as_u64().is_some_and(|b| b > 0), "{key}: {stats}");
    }

    for kind in ["images", "cardData", "models"] {
        assert_eq!(
            app.invoke("clear_cache", json!({ "kind": kind })),
            Ok(Value::Null)
        );
    }
}

#[test]
fn sync_card_db_emits_three_progress_events_one_per_stage() {
    let app = TestApp::new();
    let events = app.record("card-db-sync-progress");

    assert_eq!(app.invoke("sync_card_db", json!({})), Ok(Value::Null));

    assert_eq!(
        *events.lock().expect("not poisoned"),
        [
            json!({ "stage": "checkingVersion", "done": 1, "total": 1 }),
            json!({ "stage": "downloading", "done": 5, "total": 5 }),
            json!({ "stage": "storing", "done": 3, "total": 3 }),
        ]
    );
}

fn frame_headers(width: &str, height: &str, time_ms: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in [
        (CAMERA_ID_HEADER, "cam-1"),
        (FRAME_WIDTH_HEADER, width),
        (FRAME_HEIGHT_HEADER, height),
        (FRAME_TIME_MS_HEADER, time_ms),
    ] {
        headers.insert(name, HeaderValue::from_str(value).expect("valid header"));
    }
    headers
}

fn submit_frame(app: &TestApp, rgba: Vec<u8>, headers: HeaderMap) -> Result<Value, Value> {
    app.invoke_raw("submit_frame", InvokeBody::Raw(rgba), headers)
}

fn invalid_argument(app: &TestApp, rgba: Vec<u8>, headers: HeaderMap) -> String {
    let error = submit_frame(app, rgba, headers).expect_err("submit_frame rejects");
    assert_eq!(error["kind"], "invalidArgument", "{error}");
    error["message"]
        .as_str()
        .expect("message is a string")
        .to_owned()
}

#[test]
fn submit_frame_returns_the_canned_detections_in_frame_pixels() {
    let app = TestApp::new();

    let detections = submit_frame(
        &app,
        vec![0; 200 * 100 * 4],
        frame_headers("200", "100", "1234.5"),
    )
    .expect("a valid frame is accepted");

    let detections = detections.as_array().expect("an array of detections");
    assert_eq!(detections.len(), 5);
    assert_eq!(
        detections[0],
        json!({
            "quad": [
                { "x": 20.0, "y": 55.0 },
                { "x": 50.0, "y": 55.0 },
                { "x": 50.0, "y": 85.0 },
                { "x": 20.0, "y": 85.0 },
            ],
            "detectorConfidence": 0.91,
            "candidates": [
                { "passcode": 83_764_719, "score": 0.93 },
                { "passcode": 55_144_522, "score": 0.04 },
            ],
            "recognition": {
                "kind": "recognized",
                "passcode": 83_764_719,
                "score": 0.93,
            },
        })
    );
    let kinds: Vec<&Value> = detections
        .iter()
        .map(|d| &d["recognition"]["kind"])
        .collect();
    assert_eq!(
        kinds,
        [
            "recognized",
            "recognized",
            "lowConfidence",
            "notInDeck",
            "unreadable"
        ]
    );
    assert_eq!(detections[1]["recognition"]["passcode"], 14_558_127);
    assert_eq!(detections[2]["candidates"][0]["passcode"], 55_144_522);
}

#[test]
fn submit_frame_scales_the_canned_detections_to_each_frame_size() {
    let app = TestApp::new();

    for (width, height, first_quad) in [
        (
            640,
            480,
            json!([
                { "x": 64.0, "y": 264.0 },
                { "x": 160.0, "y": 264.0 },
                { "x": 160.0, "y": 408.0 },
                { "x": 64.0, "y": 408.0 },
            ]),
        ),
        (
            1920,
            1080,
            json!([
                { "x": 192.0, "y": 594.0 },
                { "x": 480.0, "y": 594.0 },
                { "x": 480.0, "y": 918.0 },
                { "x": 192.0, "y": 918.0 },
            ]),
        ),
    ] {
        let detections = submit_frame(
            &app,
            vec![0; width * height * 4],
            frame_headers(&width.to_string(), &height.to_string(), "0"),
        )
        .expect("a valid frame is accepted");

        assert_eq!(detections[0]["quad"], first_quad, "{width}x{height}");
    }
}

#[test]
fn submit_frame_answers_frames_from_several_cameras_one_after_another() {
    let app = TestApp::new();

    for camera in ["cam-1", "cam-2", "cam-1"] {
        let mut headers = frame_headers("2", "2", "0");
        headers.insert(CAMERA_ID_HEADER, HeaderValue::from_static(camera));
        let detections = submit_frame(&app, vec![0; 16], headers).expect("accepted");
        assert_eq!(detections.as_array().map(Vec::len), Some(5), "{camera}");
    }
}

#[test]
fn submit_frame_rejects_each_missing_header() {
    let app = TestApp::new();
    for missing in [
        CAMERA_ID_HEADER,
        FRAME_WIDTH_HEADER,
        FRAME_HEIGHT_HEADER,
        FRAME_TIME_MS_HEADER,
    ] {
        let mut headers = frame_headers("2", "2", "0");
        headers.remove(missing);

        let message = invalid_argument(&app, vec![0; 16], headers);

        assert!(message.contains(missing), "{missing}: {message}");
    }
}

#[test]
fn submit_frame_rejects_non_numeric_width_height_and_time() {
    let app = TestApp::new();
    let cases = [
        (frame_headers("wide", "2", "0"), FRAME_WIDTH_HEADER),
        (frame_headers("-2", "2", "0"), FRAME_WIDTH_HEADER),
        (frame_headers("2", "2.5", "0"), FRAME_HEIGHT_HEADER),
        (frame_headers("2", "2", "soon"), FRAME_TIME_MS_HEADER),
        (frame_headers("2", "2", "NaN"), FRAME_TIME_MS_HEADER),
        (frame_headers("2", "2", "inf"), FRAME_TIME_MS_HEADER),
    ];
    for (headers, bad) in cases {
        let message = invalid_argument(&app, vec![0; 16], headers);

        assert!(message.contains(bad), "{bad}: {message}");
    }
}

#[test]
fn submit_frame_rejects_a_header_that_is_not_visible_ascii() {
    let app = TestApp::new();
    let mut headers = frame_headers("2", "2", "0");
    headers.insert(
        CAMERA_ID_HEADER,
        HeaderValue::from_bytes(b"cam-\xff").expect("obs-text is a valid header value"),
    );

    let message = invalid_argument(&app, vec![0; 16], headers);

    assert!(message.contains(CAMERA_ID_HEADER), "{message}");
}

#[test]
fn submit_frame_rejects_a_body_whose_length_does_not_match_the_dimensions() {
    let app = TestApp::new();

    let message = invalid_argument(&app, vec![0; 15], frame_headers("2", "2", "0"));

    assert!(message.contains("expected 16"), "{message}");
}

#[test]
fn submit_frame_rejects_a_json_body() {
    let app = TestApp::new();

    let error = app
        .invoke_raw(
            "submit_frame",
            InvokeBody::Json(json!({})),
            frame_headers("2", "2", "0"),
        )
        .expect_err("a JSON body is rejected");
    assert_eq!(error["kind"], "invalidArgument");
}

/// Regenerates `src/lib/bindings.ts`. CI fails if this changes the committed file.
#[test]
fn export_typescript_bindings() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/lib/bindings.ts");
    let typescript = specta_typescript::Typescript::default().header(
        "// Generated by tauri-specta from src-tauri/src/ipc. Do not edit.\n\
         // Regenerate with `cargo test -p draw-ex` after changing a command, event or type.",
    );
    super::specta_builder::<tauri::Wry>()
        .export(typescript, &path)
        .expect("the bindings export");
}
