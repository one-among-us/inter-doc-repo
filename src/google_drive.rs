use google_drive3::{DriveHub, hyper, hyper_rustls, api::Scope};
use yup_oauth2::{ServiceAccountAuthenticator, ServiceAccountKey};
use hyper::Client;
use hyper_rustls::HttpsConnectorBuilder;

pub struct GoogleDriveClient {
    hub: DriveHub<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
}

impl GoogleDriveClient {
    pub async fn new(service_account_key_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Read service account key
        let key_content = tokio::fs::read_to_string(service_account_key_path).await?;
        let key: ServiceAccountKey = serde_json::from_str(&key_content)?;

        // Create authenticator with proper scopes (matching Python: drive.readonly)
        let auth = ServiceAccountAuthenticator::builder(key)
            .build()
            .await?;

        // Create hyper client with HTTPS connector
        let https = HttpsConnectorBuilder::new()
            .with_native_roots()?
            .https_or_http()
            .enable_http1()
            .build();
        let client = Client::builder().build(https);

        // Create Drive hub
        let hub = DriveHub::new(client, auth);

        Ok(Self { hub })
    }

    /// Download a file from Google Drive by file ID
    pub async fn download_file(&self, file_id: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use hyper::body::HttpBody;

        let result = self.hub
            .files()
            .get(file_id)
            .param("alt", "media")
            .add_scope(Scope::Readonly)
            .supports_all_drives(true)
            .doit()
            .await?;

        let mut body = result.0;
        let mut buffer = Vec::new();

        while let Some(chunk) = body.data().await {
            let chunk = chunk?;
            buffer.extend_from_slice(&chunk);
        }

        Ok(buffer)
    }

    /// Download a file as a string (useful for CSV files)
    pub async fn download_file_as_string(&self, file_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        let bytes = self.download_file(file_id).await?;
        let content = String::from_utf8(bytes)?;
        Ok(content)
    }

    /// List files in a folder (or root of shared drive)
    pub async fn list_files(&self, folder_id: Option<&str>, shared_drive_id: Option<&str>) -> Result<Vec<FileInfo>, Box<dyn std::error::Error>> {
        eprintln!("list_files called with folder_id: {:?}, shared_drive_id: {:?}", folder_id, shared_drive_id);

        let mut request = self.hub.files().list();

        // Add scope (matching Python: drive.readonly)
        request = request.add_scope(Scope::Readonly);

        // Configure for shared drive (matching Python code exactly)
        if let Some(drive_id) = shared_drive_id {
            if !drive_id.is_empty() {
                eprintln!("Configuring for shared drive: {}", drive_id);
                request = request
                    .corpora("drive")
                    .drive_id(drive_id)
                    .include_items_from_all_drives(true)
                    .supports_all_drives(true);
            }
        } else {
            // For regular drive
            request = request
                .supports_all_drives(true)
                .include_items_from_all_drives(true);
        }

        // If folder_id is specified, filter by it
        if let Some(fid) = folder_id {
            let query = format!("'{}' in parents and trashed = false", fid);
            eprintln!("Using query: {}", query);
            request = request.q(&query);
        } else {
            eprintln!("No folder filter");
        }

        // Request specific fields (matching Python code)
        request = request.param("fields", "files(id,name,mimeType,size,createdTime,modifiedTime,parents)");

        // Set page size
        request = request.page_size(100);

        eprintln!("Executing Drive API request...");
        let result = request.doit().await;

        match result {
            Ok((_, file_list)) => {
                let files = file_list.files.unwrap_or_default();
                eprintln!("API returned {} files", files.len());

                // Debug: print first few file names
                for (i, file) in files.iter().take(10).enumerate() {
                    eprintln!("  File {}: {} (ID: {}, MIME: {})",
                        i+1,
                        file.name.as_ref().unwrap_or(&"<no name>".to_string()),
                        file.id.as_ref().unwrap_or(&"<no id>".to_string()),
                        file.mime_type.as_ref().unwrap_or(&"<no mime>".to_string())
                    );
                }

                let file_infos: Vec<FileInfo> = files.into_iter().map(|f| FileInfo {
                    id: f.id.unwrap_or_default(),
                    name: f.name.unwrap_or_default(),
                    mime_type: f.mime_type.unwrap_or_default(),
                    size: f.size.map(|s| s as u64),
                    created_time: f.created_time,
                    modified_time: f.modified_time,
                    parents: f.parents,
                }).collect();

                Ok(file_infos)
            }
            Err(e) => {
                eprintln!("API error: {:?}", e);
                Err(Box::new(e))
            }
        }
    }

    /// Search files by name
    pub async fn search_files(&self, name_query: &str, shared_drive_id: Option<&str>) -> Result<Vec<FileInfo>, Box<dyn std::error::Error>> {
        let mut request = self.hub.files().list();

        // Add scope
        request = request.add_scope(Scope::Readonly);

        let query = format!("name contains '{}'", name_query);
        request = request.q(&query);

        if shared_drive_id.is_some() {
            request = request
                .supports_all_drives(true)
                .include_items_from_all_drives(true);

            if let Some(drive_id) = shared_drive_id {
                if !drive_id.is_empty() {
                    request = request.corpora("drive").drive_id(drive_id);
                }
            }
        }

        request = request.param("fields", "files(id,name,mimeType,size,createdTime,modifiedTime,parents)");

        let result = request.doit().await?;

        let files = result.1.files.unwrap_or_default();
        let file_infos: Vec<FileInfo> = files.into_iter().map(|f| FileInfo {
            id: f.id.unwrap_or_default(),
            name: f.name.unwrap_or_default(),
            mime_type: f.mime_type.unwrap_or_default(),
            size: f.size.map(|s| s as u64),
            created_time: f.created_time,
            modified_time: f.modified_time,
            parents: f.parents,
        }).collect();

        Ok(file_infos)
    }

    /// Get file metadata
    pub async fn get_file_metadata(&self, file_id: &str) -> Result<FileInfo, Box<dyn std::error::Error>> {
        let result = self.hub
            .files()
            .get(file_id)
            .supports_all_drives(true)
            .param("fields", "id,name,mimeType,size,createdTime,modifiedTime,parents")
            .doit()
            .await?;

        let file = result.1;
        Ok(FileInfo {
            id: file.id.unwrap_or_default(),
            name: file.name.unwrap_or_default(),
            mime_type: file.mime_type.unwrap_or_default(),
            size: file.size.map(|s| s as u64),
            created_time: file.created_time,
            modified_time: file.modified_time,
            parents: file.parents,
        })
    }
}

/// File information structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size: Option<u64>,
    pub created_time: Option<chrono::DateTime<chrono::Utc>>,
    pub modified_time: Option<chrono::DateTime<chrono::Utc>>,
    pub parents: Option<Vec<String>>,
}
