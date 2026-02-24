# Internal Document Repository

A secure, access-controlled document management system for formal organizational documents with Google Drive integration.

## Features

- Google OAuth 2.0 authentication
- Email domain-based authorization
- Google Drive integration for document storage
- Service account authentication for Google Drive API
- JWT-based session management
- Modular architecture

## Prerequisites

- Rust (latest stable version)
- Google Cloud Project with:
  - OAuth 2.0 credentials for user authentication
  - Service account for Google Drive API access
  - Google Drive API enabled

## Setup

### 1. Google Cloud Configuration

#### OAuth 2.0 Credentials
1. Go to Google Cloud Console
2. Create OAuth 2.0 credentials
3. Add authorized redirect URI: `http://localhost:3000/callback`
4. Note your Client ID and Client Secret

#### Service Account
1. Create a service account in Google Cloud Console
2. Download the JSON key file
3. Save it as `service_account.json` in the parent directory
4. Share your Google Drive folder with the service account email

### 2. Configuration File

Create `config.toml` in the project root:

```toml
[server]
host = "0.0.0.0"
port = 3000

[jwt]
secret = "your-secret-key-change-this"
expiry_hours = 24

[google_oauth]
client_id = "your-oauth-client-id"
client_secret = "your-oauth-client-secret"
redirect_uri = "http://localhost:3000/callback"
allowed_email_domain = "oneamongus.ca"

[google_drive]
service_account_key_path = "./service_account.json"
shared_drive_id = ""  # Optional: leave empty for My Drive
documents_index_file_id = "your-csv-file-id-from-google-drive"
```

### 3. Documents CSV

Create a `documents.csv` file with the following format and upload it to Google Drive:

```csv
organ,category,number,language,file_id,created_at,updated_at
C,RES,001,cn,1abc...xyz,2026-01-06T00:04:41Z,2026-01-06T00:04:41Z
A,DEC,002,en,2def...uvw,2026-01-07T10:30:00Z,2026-01-07T10:30:00Z
```

Fields:
- `organ`: Organization unit (e.g., A, C)
- `category`: Document category (e.g., RES, DEC)
- `number`: Document number (e.g., 001, 002)
- `language`: Language code (e.g., cn, en)
- `file_id`: Google Drive file ID
- `created_at`: ISO 8601 timestamp
- `updated_at`: ISO 8601 timestamp

To get a Google Drive file ID:
1. Right-click the file in Google Drive
2. Select "Get link"
3. The file ID is the long string in the URL: `https://drive.google.com/file/d/FILE_ID_HERE/view`

### 4. Build and Run

```bash
# Check for errors
cargo check

# Build the project
cargo build

# Run the application
cargo run
```

The server will start on `http://localhost:3000`

## Usage

### Accessing Documents

Documents are accessed via semantic URLs:

```
http://localhost:3000/{organ}/{category}/{number}
```

Examples:
- `http://localhost:3000/C/RES/001`
- `http://localhost:3000/A/DEC/002`

### Browsing Google Drive Files

The application includes an admin interface to browse and search files in your Google Drive:

1. Navigate to `http://localhost:3000/admin`
2. Sign in with your authorized email
3. You can:
   - Browse folders by clicking on them
   - Search files by name
   - Copy file IDs to clipboard (for adding to documents.csv)
   - View file metadata (size, modified date, etc.)

This makes it easy to find file IDs without manually navigating Google Drive!

### Authentication Flow

1. Navigate to a document URL or admin page
2. If not authenticated, you'll be redirected to Google OAuth
3. Sign in with an email from the allowed domain
4. You'll be redirected back to the original page
5. Documents will be streamed from Google Drive

## Project Structure

```
src/
├── main.rs          # Application entry point
├── config.rs        # Configuration management
├── auth.rs          # Authentication and authorization
├── google_drive.rs  # Google Drive API integration
├── document.rs      # Document management
└── admin.rs         # Admin interface for browsing Drive

config.toml          # Configuration file (not in git)
service_account.json     # Service account key (not in git)
```

## Security Notes

**CRITICAL: Before pushing to GitHub, ensure these files are NOT committed:**

### Protected Files (already in .gitignore)
- `config.toml` - Contains all sensitive credentials
- `service_account.json` - Google service account private key

### Sensitive Parameters in config.toml
- `jwt.secret` - JWT signing key (use strong random string)
- `google_oauth.client_id` - OAuth client ID
- `google_oauth.client_secret` - OAuth client secret
- `google_drive.shared_drive_id` - Your Drive ID
- `google_drive.documents_index_file_id` - Your Sheets ID

### Setup for New Deployments
1. Copy `config.toml.example` to `config.toml`
2. Replace all placeholder values with your actual credentials
3. Generate a strong JWT secret: `openssl rand -base64 32`
4. Never commit the actual `config.toml` or `service_account.json`

### Production Security Checklist
- [ ] Use HTTPS (not HTTP)
- [ ] Generate strong JWT secret (32+ characters)
- [ ] Regularly rotate service account keys
- [ ] Review and limit service account permissions
- [ ] Enable Google Cloud audit logging
- [ ] Use environment variables for secrets (optional alternative to config.toml)

## Development

### Using the Admin Interface

The admin interface (`/admin`) is the easiest way to manage your documents:

1. **Browse Files**: Navigate through your Google Drive folder structure
2. **Search**: Find files by name using the search box
3. **Copy File IDs**: Click the "Copy ID" button next to any file to copy its ID
4. **Add to CSV**: Use the copied file ID in your `documents.csv`

### Adding New Documents

**Method 1: Using Admin Interface (Recommended)**
1. Upload the PDF to Google Drive
2. Share it with the service account (if not in shared drive)
3. Navigate to `http://localhost:3000/admin`
4. Search or browse to find your file
5. Click "Copy ID" to copy the file ID
6. Add an entry to `documents.csv` on Google Drive with the file ID
7. Restart the application to reload the document list

**Method 2: Manual**
1. Upload the PDF to Google Drive
2. Share it with the service account
3. Get the file ID from the URL
4. Add an entry to `documents.csv` on Google Drive
5. Restart the application to reload the document list

### Modifying Configuration

Edit `config.toml` and restart the application.

## Troubleshooting

### "Failed to load config.toml"
- Ensure `config.toml` exists in the project root
- Check file permissions

### "Failed to initialize Google Drive client"
- Verify `service_account.json` path is correct
- Ensure the service account has access to the files
- Check that Google Drive API is enabled

### "Document not found"
- Verify the document exists in `documents.csv`
- Check that the file ID is correct
- Ensure the service account has access to the file

### Authentication fails
- Verify OAuth credentials are correct
- Check that the redirect URI matches
- Ensure the user's email domain is allowed
