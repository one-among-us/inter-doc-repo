# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Internal Document Repository (受控访问文档系统) - A secure, access-controlled document management system for formal organizational documents with structured numbering. Built with Rust using Axum web framework with Google Drive and Google Sheets integration.

## Key Requirements

- **Authentication**: Google OAuth 2.0 with automatic redirect on invalid/missing tokens
- **Authorization**: Email domain must be `@oneamongus.ca` and `email_verified = true`
- **Access Control**: Unauthenticated users are automatically redirected to login with return URL preserved
- **Storage**: Documents stored on Google Drive, metadata in Google Sheets
- **URL Structure**: Semantic URLs following pattern `/{Organ}/{Category}/{Number}`
  - Example: `A/RES/001`, `C/DEC/014`

## Configuration

All sensitive credentials and configuration are stored in `config.toml` (not committed to git):

```toml
[server]
host = "0.0.0.0"
port = 3000

[jwt]
secret = "your-secret-key"
expiry_hours = 24

[google_oauth]
client_id = "your-client-id"
client_secret = "your-client-secret"
redirect_uri = "http://localhost:3000/callback"

[google_drive]
service_account_key_path = "./service_account.json"
shared_drive_id = "your-shared-drive-id"
documents_index_file_id = "your-google-sheets-spreadsheet-id"
```

**Note**: `documents_index_file_id` is now a Google Sheets spreadsheet ID, not a CSV file ID.

## Common Commands

```bash
# Check for errors without building
cargo check

# Build the project
cargo build

# Run the application
cargo run

# Run tests
cargo test
```

## Architecture

### Module Structure

- `src/main.rs`: Application entry point, initializes clients and configures routes
- `src/config.rs`: Configuration loading from config.toml
- `src/auth.rs`: Google OAuth 2.0 and JWT authentication/authorization
- `src/google_drive.rs`: Google Drive API integration (list, search, download files)
- `src/google_sheets.rs`: Google Sheets API integration (read spreadsheet data)
- `src/document.rs`: Document management and serving
- `src/admin.rs`: Admin interface for browsing Google Drive

### Authentication Flow

1. User accesses protected route (e.g., `/C/RES/001` or `/admin`)
2. `auth_middleware` checks for valid JWT token in cookies
3. If missing/invalid: redirect to `/login?return_to=/C/RES/001`
4. Login redirects to Google OAuth with `state` parameter containing return URL
5. Google redirects to `/callback?code=...&state=/C/RES/001`
6. Callback exchanges code for access token, validates email domain (`@oneamongus.ca`)
7. Creates JWT with `aud: "app"`, sets cookie via JavaScript (not HttpOnly due to redirect issues)
8. Redirects user back to original URL

**Critical**: Cookie must be set via JavaScript in HTML response, not via Set-Cookie header in redirect response, as browsers may not persist cookies during 303 redirects.

### Document Storage and Retrieval

Documents are stored on Google Drive with metadata in Google Sheets:

1. At startup, document metadata is read from Google Sheets (spreadsheet ID in config)
2. Sheet format (first row is header):
```
organ,category,number,language,file_id,created_at,updated_at
C,RES,001,cn,1abc...xyz,2026-01-06T00:04:41Z,2026-01-06T00:04:41Z
```
3. Data is loaded into in-memory HashMap: `{organ}/{category}/{number}` → Document
4. When a document is requested, it's streamed directly from Google Drive using `file_id`
5. No local file storage is used

**Note**: `file_id` is the Google Drive file ID, not a local path

### Routes

- `/login` - Initiates Google OAuth flow, accepts `return_to` query parameter
- `/callback` - OAuth callback handler (NOT protected by auth middleware)
- `/health` - Health check endpoint
- `/admin` - Admin UI for browsing Google Drive (protected by auth middleware)
- `/admin/browse?folder_id=xxx` - API endpoint to list files in a folder (protected)
- `/admin/search?q=xxx` - API endpoint to search files by name (protected)
- `/{organ}/{category}/{number}` - Document access (protected by auth middleware)

### JWT Token Details

- Algorithm: HS256
- Secret: Configured in config.toml
- Audience: "app" (must match in both token creation and validation)
- Expiry: Configurable in config.toml (default 24 hours)
- Validation requires: `validation.set_audience(&["app"])`

### Google API Integration

**Google Drive API**:
- Uses service account authentication with `Scope::Readonly`
- Service account key stored in `service_account.json` (not committed to git)
- Supports shared drives via `corpora("drive")` and `drive_id()`
- Downloads files on-demand (no caching)
- **Critical**: Must call `.add_scope(Scope::Readonly)` on each API request

**Google Sheets API**:
- Uses same service account as Drive
- Reads spreadsheet data with `Scope::SpreadsheetReadonly`
- Default range: "A:Z" (reads all columns from first sheet)
- Converts sheet data to CSV format for parsing

**Admin Interface**:
- Browse and search files in Google Drive via web UI
- List files in folders (supports shared drives)
- Search files by name
- View file metadata (size, modified date, MIME type)
- Copy file IDs to clipboard for easy reference

### Application State Management

The application uses multiple state structs for different route groups:

- `AuthState`: Shared by login/callback routes and auth middleware
  - Contains: HTTP client, config
- `DocumentState`: Used by document serving routes
  - Contains: Document HashMap, Drive client
- `AdminState`: Used by admin interface routes
  - Contains: Drive client, config

This separation allows different route groups to have access to only the state they need.

## Development Notes

- The project uses Rust edition 2021
- All sensitive credentials are in `config.toml` and `service_account.json` (both gitignored)
- Document metadata is loaded from Google Sheets at startup into in-memory HashMap
- Documents are streamed from Google Drive on each request (no caching)
- Cookie is set via JavaScript to avoid browser redirect cookie persistence issues
- Google API scopes must be explicitly added to each request using `.add_scope()`

## Google API Scope Requirements

When working with Google APIs, always add the appropriate scope to requests:

```rust
// Google Drive
request.add_scope(google_drive3::api::Scope::Readonly)

// Google Sheets
request.add_scope(google_sheets4::api::Scope::SpreadsheetReadonly)
```

Without explicit scopes, API calls will return 0 results even with proper authentication.

## Setup Requirements

1. **Google Cloud Project**:
   - OAuth 2.0 credentials for user authentication
   - Service account with JSON key file
   - Google Drive API enabled
   - Google Sheets API enabled

2. **Service Account Permissions**:
   - Add service account email to Shared Drive as "Viewer" or higher
   - Share Google Sheets spreadsheet with service account email

3. **Configuration Files**:
   - `config.toml` with all credentials and IDs
   - `service_account.json` with service account key

4. **Google Sheets Format**:
   - First row must be header: `organ,category,number,language,file_id,created_at,updated_at`
   - Each subsequent row is a document entry
   - Can be edited directly in Google Sheets (restart app to reload)
