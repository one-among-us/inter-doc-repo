use axum::{
    http::StatusCode,
    response::{IntoResponse, Html},
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use crate::google_drive::GoogleDriveClient;
use crate::config::Config;

// Admin state
#[derive(Clone)]
pub struct AdminState {
    pub drive_client: Arc<GoogleDriveClient>,
    pub config: Config,
}

#[derive(Debug, Deserialize)]
pub struct BrowseQuery {
    folder_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    q: String,
}

/// Browse files in a folder
pub async fn browse_files(
    State(state): State<AdminState>,
    Query(params): Query<BrowseQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let folder_id = params.folder_id.as_deref();
    let shared_drive_id = if state.config.google_drive.shared_drive_id.is_empty() {
        None
    } else {
        Some(state.config.google_drive.shared_drive_id.as_str())
    };

    eprintln!("Browse request - folder_id: {:?}, shared_drive_id: {:?}", folder_id, shared_drive_id);

    match state.drive_client.list_files(folder_id, shared_drive_id).await {
        Ok(files) => {
            eprintln!("Successfully listed {} files", files.len());
            Ok(Json(files).into_response())
        },
        Err(e) => {
            eprintln!("Failed to list files: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to list files: {}", e)))
        }
    }
}

/// Search files by name
pub async fn search_files(
    State(state): State<AdminState>,
    Query(params): Query<SearchQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let shared_drive_id = if state.config.google_drive.shared_drive_id.is_empty() {
        None
    } else {
        Some(state.config.google_drive.shared_drive_id.as_str())
    };

    match state.drive_client.search_files(&params.q, shared_drive_id).await {
        Ok(files) => Ok(Json(files).into_response()),
        Err(e) => {
            eprintln!("Failed to search files: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to search files: {}", e)))
        }
    }
}

/// Admin UI for browsing files
pub async fn admin_ui() -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Google Drive Browser</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }
        h1 {
            color: #333;
        }
        .search-box {
            margin: 20px 0;
        }
        .search-box input {
            padding: 10px;
            width: 300px;
            font-size: 14px;
        }
        .search-box button {
            padding: 10px 20px;
            font-size: 14px;
            cursor: pointer;
        }
        .breadcrumb {
            margin: 20px 0;
            padding: 10px;
            background: #f0f0f0;
            border-radius: 4px;
        }
        .file-list {
            border: 1px solid #ddd;
            border-radius: 4px;
        }
        .file-item {
            padding: 12px;
            border-bottom: 1px solid #eee;
            display: flex;
            align-items: center;
            cursor: pointer;
        }
        .file-item:hover {
            background: #f9f9f9;
        }
        .file-item:last-child {
            border-bottom: none;
        }
        .file-icon {
            margin-right: 10px;
            font-size: 20px;
        }
        .file-info {
            flex: 1;
        }
        .file-name {
            font-weight: bold;
            color: #0066cc;
        }
        .file-meta {
            font-size: 12px;
            color: #666;
            margin-top: 4px;
        }
        .file-id {
            font-family: monospace;
            font-size: 11px;
            color: #999;
            margin-top: 4px;
        }
        .copy-btn {
            padding: 4px 8px;
            font-size: 11px;
            cursor: pointer;
            background: #4CAF50;
            color: white;
            border: none;
            border-radius: 3px;
        }
        .copy-btn:hover {
            background: #45a049;
        }
        .loading {
            text-align: center;
            padding: 40px;
            color: #666;
        }
        .error {
            padding: 20px;
            background: #ffebee;
            color: #c62828;
            border-radius: 4px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <h1>📁 Google Drive Browser</h1>

    <div class="search-box">
        <input type="text" id="searchInput" placeholder="Search files by name...">
        <button onclick="searchFiles()">Search</button>
        <button onclick="loadFiles()">Show All</button>
    </div>

    <div class="breadcrumb" id="breadcrumb">
        <span onclick="loadFiles()">🏠 Root</span>
    </div>

    <div id="content">
        <div class="loading">Loading...</div>
    </div>

    <script>
        let currentFolder = null;

        async function loadFiles(folderId = null) {
            currentFolder = folderId;
            const content = document.getElementById('content');
            content.innerHTML = '<div class="loading">Loading...</div>';

            try {
                const url = folderId
                    ? `/admin/browse?folder_id=${folderId}`
                    : '/admin/browse';

                const response = await fetch(url);
                if (!response.ok) throw new Error('Failed to load files');

                const files = await response.json();
                displayFiles(files);
            } catch (error) {
                content.innerHTML = `<div class="error">Error: ${error.message}</div>`;
            }
        }

        async function searchFiles() {
            const query = document.getElementById('searchInput').value;
            if (!query) return;

            const content = document.getElementById('content');
            content.innerHTML = '<div class="loading">Searching...</div>';

            try {
                const response = await fetch(`/admin/search?q=${encodeURIComponent(query)}`);
                if (!response.ok) throw new Error('Search failed');

                const files = await response.json();
                displayFiles(files);
            } catch (error) {
                content.innerHTML = `<div class="error">Error: ${error.message}</div>`;
            }
        }

        function displayFiles(files) {
            const content = document.getElementById('content');

            if (files.length === 0) {
                content.innerHTML = '<div class="loading">No files found</div>';
                return;
            }

            const fileList = document.createElement('div');
            fileList.className = 'file-list';

            files.forEach(file => {
                const item = document.createElement('div');
                item.className = 'file-item';

                const icon = getFileIcon(file.mime_type);
                const size = file.size ? formatSize(file.size) : 'N/A';
                const modified = file.modified_time ? new Date(file.modified_time).toLocaleString() : 'N/A';

                item.innerHTML = `
                    <div class="file-icon">${icon}</div>
                    <div class="file-info">
                        <div class="file-name">${escapeHtml(file.name)}</div>
                        <div class="file-meta">Size: ${size} | Modified: ${modified}</div>
                        <div class="file-id">
                            ID: ${file.id}
                            <button class="copy-btn" onclick="copyToClipboard('${file.id}')">Copy ID</button>
                        </div>
                    </div>
                `;

                if (file.mime_type === 'application/vnd.google-apps.folder') {
                    item.onclick = () => loadFiles(file.id);
                }

                fileList.appendChild(item);
            });

            content.innerHTML = '';
            content.appendChild(fileList);
        }

        function getFileIcon(mimeType) {
            if (mimeType === 'application/vnd.google-apps.folder') return '📁';
            if (mimeType === 'application/pdf') return '📄';
            if (mimeType.startsWith('image/')) return '🖼️';
            if (mimeType.startsWith('video/')) return '🎥';
            if (mimeType.startsWith('audio/')) return '🎵';
            if (mimeType.includes('spreadsheet')) return '📊';
            if (mimeType.includes('document')) return '📝';
            return '📎';
        }

        function formatSize(bytes) {
            if (bytes < 1024) return bytes + ' B';
            if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
            if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
            return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB';
        }

        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        function copyToClipboard(text) {
            navigator.clipboard.writeText(text).then(() => {
                alert('File ID copied to clipboard!');
            });
        }

        // Load files on page load
        loadFiles();

        // Search on Enter key
        document.getElementById('searchInput').addEventListener('keypress', (e) => {
            if (e.key === 'Enter') searchFiles();
        });
    </script>
</body>
</html>"#;

    Html(html)
}
