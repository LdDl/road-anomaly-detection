use super::{ApiState, handlers};
use crate::app::RestApiSettings;
use actix_files::Files;
use actix_web::{http::StatusCode, middleware::DefaultHeaders, web, App, HttpServer, ResponseError};
use std::io;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use tokio::sync::oneshot;

pub struct ApiServer {
    stop: Option<oneshot::Sender<()>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Drop for ApiServer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn routes(config: &mut web::ServiceConfig) {
    config.service(web::scope("/api")
        .route("/ping", web::get().to(handlers::ping))
        .route("/health", web::get().to(handlers::health))
        .route("/zones", web::get().to(handlers::get_zones))
        .route("/zones", web::post().to(handlers::update_zones))
        .route("/frame", web::get().to(handlers::frame))
        .route("/mutations/zones/create", web::post().to(handlers::create_zone))
        .route("/mutations/zones/update", web::post().to(handlers::update_zone))
        .route("/mutations/zones/delete", web::post().to(handlers::delete_zone))
        .route("/mutations/save_config", web::post().to(handlers::save_config))
        .default_service(web::to(|| async { handlers::error(StatusCode::NOT_FOUND, "API endpoint not found") })))
        .route("/live_streaming", web::get().to(handlers::mjpeg_stream))
        .route("/api-docs/openapi.json", web::get().to(handlers::openapi));
}

pub fn start(settings: &RestApiSettings, state: Arc<ApiState>) -> io::Result<ApiServer> {
    let listener = TcpListener::bind((settings.host.as_str(), settings.port))?;
    let address = listener.local_addr()?;
    let web_ui_dir = settings.web_ui_dir.clone();
    let serve_ui = std::path::Path::new(&web_ui_dir).join("index.html").is_file();
    if !serve_ui {
        eprintln!("Web UI not found at {}. Build it with: cd web && npm ci && npm run build", web_ui_dir);
    }
    let (stop, stopped) = oneshot::channel();
    let worker = thread::Builder::new().name("rest-api".to_string()).spawn(move || {
        let result = actix_web::rt::System::new().block_on(async move {
            let state = web::Data::from(state);
            let server = HttpServer::new(move || {
                let app = App::new()
                    .wrap(DefaultHeaders::new().add(("Cache-Control", "no-store")))
                    .app_data(state.clone())
                    .app_data(web::JsonConfig::default().limit(262144).error_handler(|err, _| {
                        let response = handlers::error(err.status_code(), err.to_string());
                        actix_web::error::InternalError::from_response(err, response).into()
                    }))
                    .configure(routes);
                if serve_ui {
                    app.service(Files::new("/", &web_ui_dir).index_file("index.html"))
                } else {
                    app.default_service(web::to(|| async { handlers::error(StatusCode::SERVICE_UNAVAILABLE, "Web interface is unavailable") }))
                }
            })
                .workers(1)
                .worker_max_blocking_threads(2)
                .disable_signals()
                .shutdown_timeout(1)
                .listen(listener)?
                .run();
            let handle = server.handle();
            actix_web::rt::spawn(async move {
                let _ = stopped.await;
                handle.stop(false).await;
            });
            server.await
        });
        if let Err(err) = result {
            eprintln!("REST API stopped: {}", err);
        }
    })?;
    println!("Web UI and REST API: http://{}", address);
    Ok(ApiServer { stop: Some(stop), worker: Some(worker) })
}
