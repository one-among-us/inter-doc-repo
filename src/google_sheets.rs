use google_sheets4::{Sheets, hyper, hyper_rustls, api::Scope};
use yup_oauth2::{ServiceAccountAuthenticator, ServiceAccountKey};
use hyper::Client;
use hyper_rustls::HttpsConnectorBuilder;

pub struct GoogleSheetsClient {
    hub: Sheets<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
}

impl GoogleSheetsClient {
    pub async fn new(service_account_key_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Read service account key
        let key_content = tokio::fs::read_to_string(service_account_key_path).await?;
        let key: ServiceAccountKey = serde_json::from_str(&key_content)?;

        // Create authenticator
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

        // Create Sheets hub
        let hub = Sheets::new(client, auth);

        Ok(Self { hub })
    }

    /// Read data from a Google Sheet
    /// Returns a Vec of rows, where each row is a Vec of cell values
    pub async fn read_sheet(&self, spreadsheet_id: &str, range: &str) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
        eprintln!("Reading sheet: {} range: {}", spreadsheet_id, range);

        let result = self.hub
            .spreadsheets()
            .values_get(spreadsheet_id, range)
            .add_scope(Scope::SpreadsheetReadonly)
            .doit()
            .await?;

        let values = result.1.values.unwrap_or_default();

        eprintln!("Read {} rows from sheet", values.len());

        // Convert from serde_json::Value to String
        let rows: Vec<Vec<String>> = values
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| {
                        match cell {
                            serde_json::Value::String(s) => s,
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            serde_json::Value::Null => String::new(),
                            _ => cell.to_string(),
                        }
                    })
                    .collect()
            })
            .collect();

        Ok(rows)
    }

    /// Read sheet and convert to CSV format
    pub async fn read_sheet_as_csv(&self, spreadsheet_id: &str, range: &str) -> Result<String, Box<dyn std::error::Error>> {
        let rows = self.read_sheet(spreadsheet_id, range).await?;

        let mut csv_content = String::new();
        for row in rows {
            let line = row.join(",");
            csv_content.push_str(&line);
            csv_content.push('\n');
        }

        Ok(csv_content)
    }
}
