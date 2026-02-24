use axum::{
    http::{StatusCode, header, HeaderMap},
    response::IntoResponse,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use crate::google_drive::GoogleDriveClient;

// Document metadata structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Document {
    pub organ: String,
    pub category: String,
    pub number: String,
    pub language: String,
    pub created_at: String,
    pub updated_at: String,
    pub file_id: String,  // Google Drive file ID instead of local file path
}

// Application state for document management
#[derive(Clone)]
pub struct DocumentState {
    pub documents: Arc<HashMap<String, Document>>,
    pub drive_client: Arc<GoogleDriveClient>,
}

/// Load documents from CSV content (from Google Drive)
pub fn load_documents_from_csv(csv_content: &str) -> Result<HashMap<String, Document>, Box<dyn std::error::Error>> {
    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());
    let mut docs = HashMap::new();

    for result in reader.deserialize() {
        let record: Document = result?;
        let key = format!("{}/{}/{}", record.organ, record.category, record.number);
        docs.insert(key, record);
    }

    Ok(docs)
}

/// List all documents handler - displays all available documents
pub async fn list_documents(
    State(state): State<DocumentState>,
) -> impl IntoResponse {
    let mut docs: Vec<_> = state.documents.iter().collect();
    docs.sort_by(|a, b| a.0.cmp(b.0));

    let mut html = String::from(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Internal Document Repository</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 1200px; margin: 40px auto; padding: 0 20px; }
        h1 { color: #333; }
        table { width: 100%; border-collapse: collapse; margin-top: 20px; }
        th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }
        th { background-color: #f5f5f5; font-weight: bold; }
        tr:hover { background-color: #f9f9f9; }
        a { color: #0066cc; text-decoration: none; }
        a:hover { text-decoration: underline; }
        .empty { text-align: center; padding: 40px; color: #666; }
    </style>
</head>
<body>
    <h1>Internal Document Repository</h1>
"#);

    if docs.is_empty() {
        html.push_str(r#"    <div class="empty">
        <p>No documents available.</p>
        <p>Visit <a href="/admin">/admin</a> to browse Google Drive and configure documents.</p>
    </div>
"#);
    } else {
        html.push_str(r#"    <table>
        <thead>
            <tr>
                <th>Document ID</th>
                <th>Organ</th>
                <th>Category</th>
                <th>Number</th>
                <th>Language</th>
                <th>Created</th>
                <th>Updated</th>
            </tr>
        </thead>
        <tbody>
"#);

        for (key, doc) in docs {
            html.push_str(&format!(
                r#"            <tr>
                <td><a href="/{}">{}</a></td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>
"#,
                key, key, doc.organ, doc.category, doc.number, doc.language, doc.created_at, doc.updated_at
            ));
        }

        html.push_str(r#"        </tbody>
    </table>
"#);
    }

    html.push_str(r#"</body>
</html>"#);

    (StatusCode::OK, [("content-type", "text/html; charset=utf-8")], html)
}

/// Document access handler - streams PDF from Google Drive
pub async fn get_document(
    Path((organ, category, number)): Path<(String, String, String)>,
    State(state): State<DocumentState>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    // Create document ID
    let doc_id = format!("{}/{}/{}", organ, category, number);

    // Look up document in storage
    let document = match state.documents.get(&doc_id) {
        Some(doc) => doc,
        None => return Err((StatusCode::NOT_FOUND, "Document not found")),
    };

    // Download file from Google Drive
    let file_data = match state.drive_client.download_file(&document.file_id).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to download file from Google Drive: {:?}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to retrieve document"));
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/pdf".parse().unwrap());

    Ok((StatusCode::OK, headers, file_data))
}
