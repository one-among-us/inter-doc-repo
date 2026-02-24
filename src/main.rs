mod config;
mod google_drive;
mod google_sheets;
mod auth;
mod document;
mod admin;

use axum::{
    routing::get,
    Router,
    http::StatusCode,
    response::IntoResponse,
    middleware,
};
use std::sync::Arc;
use std::net::SocketAddr;

use config::Config;
use google_drive::GoogleDriveClient;
use google_sheets::GoogleSheetsClient;
use auth::{AuthState, login, callback, auth_middleware};
use document::{DocumentState, load_documents_from_csv, get_document, list_documents};
use admin::{AdminState, admin_ui, browse_files, search_files};

/// Health check endpoint
async fn health() -> impl IntoResponse {
    (StatusCode::OK, "Server is healthy")
}

#[tokio::main]
async fn main() {
    // Load configuration
    let config = Config::load("config.toml")
        .expect("Failed to load config.toml");

    println!("Configuration loaded successfully");

    // Initialize Google Drive client
    let drive_client = GoogleDriveClient::new(&config.google_drive.service_account_key_path)
        .await
        .expect("Failed to initialize Google Drive client");

    println!("Google Drive client initialized");

    // Initialize Google Sheets client
    let sheets_client = GoogleSheetsClient::new(&config.google_drive.service_account_key_path)
        .await
        .expect("Failed to initialize Google Sheets client");

    println!("Google Sheets client initialized");

    // Load documents from Google Sheets (optional - if spreadsheet ID is empty, start with empty list)
    let documents = if config.google_drive.documents_index_file_id.is_empty() {
        println!("No spreadsheet ID configured, starting with empty document list");
        println!("Visit http://localhost:3000/admin to browse your Drive and find files");
        std::collections::HashMap::new()
    } else {
        // Try to read from Google Sheets
        // Assume the spreadsheet ID is in documents_index_file_id
        // Use just the spreadsheet ID without specifying sheet name - will read first sheet
        let spreadsheet_id = &config.google_drive.documents_index_file_id;
        let range = "A:Z"; // Read all columns from first sheet (no sheet name specified)

        match sheets_client.read_sheet_as_csv(spreadsheet_id, range).await {
            Ok(csv_content) => {
                println!("Downloaded data from Google Sheets");
                match load_documents_from_csv(&csv_content) {
                    Ok(docs) => {
                        println!("Loaded {} documents", docs.len());
                        docs
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse sheet data: {}", e);
                        println!("Starting with empty document list");
                        std::collections::HashMap::new()
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to read from Google Sheets: {}", e);
                println!("Starting with empty document list");
                println!("Visit http://localhost:3000/admin to browse your Drive");
                std::collections::HashMap::new()
            }
        }
    };

    // Create HTTP client for OAuth
    let http_client = reqwest::Client::new();

    // Create application states
    let auth_state = AuthState {
        client: http_client.clone(),
        config: config.clone(),
    };

    let doc_state = DocumentState {
        documents: Arc::new(documents),
        drive_client: Arc::clone(&Arc::new(drive_client)),
    };

    let admin_state = AdminState {
        drive_client: Arc::clone(&doc_state.drive_client),
        config: config.clone(),
    };

    // Build admin routes (protected by auth middleware)
    let admin_routes = Router::new()
        .route("/admin", get(admin_ui))
        .route("/admin/browse", get(browse_files))
        .route("/admin/search", get(search_files))
        .with_state(admin_state)
        .layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware));

    // Build protected routes with auth middleware
    let protected = Router::new()
        .route("/", get(list_documents))
        .route("/:organ/:category/:number", get(get_document))
        .with_state(doc_state)
        .layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware));

    // Build main application
    let app = Router::new()
        .route("/login", get(login))
        .route("/callback", get(callback))
        .route("/health", get(health))
        .with_state(auth_state)
        .merge(admin_routes)
        .merge(protected);

    // Run server
    let addr = SocketAddr::from((
        config.server.host.parse::<std::net::IpAddr>().unwrap_or([0, 0, 0, 0].into()),
        config.server.port
    ));

    println!("Server is listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}
