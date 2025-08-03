# Integration Tools

Tools for integrating with external services and APIs.

## Tools

### `google_sheets_tools.py`

**Purpose**: Google Sheets integration using service account authentication.

**Usage**:
```bash
python3 google_sheets_tools.py <operation> [arguments...]
```

**Operations**:

#### Read Data
```bash
# Read specific range
python3 google_sheets_tools.py read "spreadsheet_id" "Sheet1!A1:C10"

# Read entire sheet
python3 google_sheets_tools.py read "spreadsheet_id" "Sheet1"

# Read multiple ranges
python3 google_sheets_tools.py read "spreadsheet_id" "Sheet1!A:A,Sheet1!C:C"
```

#### Write Data
```bash
# Write to specific range
python3 google_sheets_tools.py write "spreadsheet_id" "Sheet1!A1" "Hello,World"

# Write multiple rows
python3 google_sheets_tools.py write "spreadsheet_id" "Sheet1!A1:B2" "A1,B1;A2,B2"
```

#### List Operations
```bash
# List all accessible spreadsheets
python3 google_sheets_tools.py list_spreadsheets

# List sheets in a spreadsheet
python3 google_sheets_tools.py list_sheets "spreadsheet_id"
```

#### Create Operations
```bash
# Create new spreadsheet
python3 google_sheets_tools.py create_spreadsheet "My New Sheet"

# Create new sheet in existing spreadsheet
python3 google_sheets_tools.py create_sheet "spreadsheet_id" "New Sheet Name"
```

### `google_sheets_oauth_tools.py`

**Purpose**: Google Sheets integration using OAuth2 authentication for user access.

**Usage**:
```bash
python3 google_sheets_oauth_tools.py <operation> [arguments...]
```

**Operations** (same as service account version):
- `read` - Read data from sheets
- `write` - Write data to sheets  
- `list_spreadsheets` - List user's spreadsheets
- `list_sheets` - List sheets in spreadsheet
- `create_spreadsheet` - Create new spreadsheet
- `create_sheet` - Create new sheet

**Authentication Flow**:
1. First run opens browser for OAuth consent
2. User grants permissions
3. Token saved locally for future use
4. Subsequent runs use saved token

## Authentication Setup

### Service Account (Production)

1. **Create Service Account**:
   - Go to Google Cloud Console
   - Enable Google Sheets API
   - Create service account
   - Download JSON key file

2. **Environment Setup**:
   ```bash
   export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account.json"
   ```

3. **Share Spreadsheets**:
   - Share target spreadsheets with service account email
   - Grant appropriate permissions (Viewer/Editor)

### OAuth2 (Development)

1. **Create OAuth Credentials**:
   - Go to Google Cloud Console
   - Enable Google Sheets API
   - Create OAuth 2.0 client credentials
   - Download credentials JSON

2. **Environment Setup**:
   ```bash
   export GOOGLE_OAUTH_CREDENTIALS_FILE="/path/to/credentials.json"
   export GOOGLE_OAUTH_TOKEN_FILE="/path/to/token.pickle"
   ```

3. **First Run**:
   ```bash
   python3 google_sheets_oauth_tools.py list_spreadsheets
   # Browser opens for consent
   # Token saved automatically
   ```

## Integration Examples

### With MagicTunnel Capabilities

The tools are integrated into capability files:

```yaml
# capabilities/google/google_sheets.yaml
tools:
  - name: "read_google_sheet"
    description: "Read data from Google Sheets"
    routing:
      type: "subprocess"
      command: "python3"
      args: ["tools/integrations/google_sheets_tools.py", "read", "{spreadsheet_id}", "{range}"]
    input_schema:
      type: "object"
      properties:
        spreadsheet_id:
          type: "string"
          description: "Google Sheets spreadsheet ID"
        range:
          type: "string"
          description: "Range to read (e.g., 'Sheet1!A1:C10')"
      required: ["spreadsheet_id", "range"]
```

### Batch Operations

```bash
# Process multiple spreadsheets
cat spreadsheet_ids.txt | while read id; do
    python3 google_sheets_tools.py read "$id" "Sheet1!A:Z" > "data_$id.csv"
done

# Backup all user spreadsheets
python3 google_sheets_oauth_tools.py list_spreadsheets | \
    jq -r '.files[].id' | \
    while read id; do
        python3 google_sheets_oauth_tools.py read "$id" "Sheet1" > "backup_$id.csv"
    done
```

### Data Processing Pipeline

```bash
#!/bin/bash
# Complete data processing pipeline

# 1. Extract data from Google Sheets
python3 tools/integrations/google_sheets_tools.py read \
    "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms" \
    "Class Data!A:E" > raw_data.csv

# 2. Process data (example)
python3 -c "
import pandas as pd
df = pd.read_csv('raw_data.csv')
processed = df.groupby('Category').sum()
processed.to_csv('processed_data.csv')
"

# 3. Upload results back to sheets
python3 tools/integrations/google_sheets_tools.py write \
    "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms" \
    "Results!A1" \
    "$(cat processed_data.csv)"
```

## Error Handling

### Common Issues

1. **Authentication Errors**
   ```
   Error: Service account key not found
   Fix: Set GOOGLE_APPLICATION_CREDENTIALS environment variable
   ```

2. **Permission Errors**
   ```
   Error: The caller does not have permission
   Fix: Share spreadsheet with service account email
   ```

3. **OAuth Token Expired**
   ```
   Error: Token expired
   Fix: Delete token file and re-authenticate
   ```

4. **API Quota Exceeded**
   ```
   Error: Quota exceeded
   Fix: Implement rate limiting or request quota increase
   ```

### Rate Limiting

```python
# Built-in rate limiting in tools
import time

def rate_limited_request(func, *args, **kwargs):
    """Rate-limited API request with backoff"""
    max_retries = 3
    for attempt in range(max_retries):
        try:
            return func(*args, **kwargs)
        except QuotaExceeded:
            if attempt < max_retries - 1:
                time.sleep(2 ** attempt)  # Exponential backoff
            else:
                raise
```

## Output Formats

### CSV Format
```bash
# Read as CSV (default)
python3 google_sheets_tools.py read "id" "A1:C3"
# Output: "A1,B1,C1\nA2,B2,C2\nA3,B3,C3"
```

### JSON Format
```bash
# Read as JSON
python3 google_sheets_tools.py read "id" "A1:C3" --format json
# Output: [["A1","B1","C1"],["A2","B2","C2"],["A3","B3","C3"]]
```

### Raw Values
```bash
# Read raw values only
python3 google_sheets_tools.py read "id" "A1:C3" --values-only
# Output: Raw cell values without formatting
```

## Security Considerations

### Service Account Security
- Store service account keys securely
- Use least privilege principle
- Rotate keys regularly
- Monitor API usage

### OAuth2 Security
- Store tokens securely
- Use HTTPS for redirect URIs
- Implement token refresh logic
- Handle consent revocation

### Data Security
- Encrypt sensitive data in transit
- Validate all input data
- Implement audit logging
- Use secure communication channels

## Performance Optimization

### Batch Requests
```python
# Batch multiple operations
batch_requests = [
    {"range": "Sheet1!A1:A10"},
    {"range": "Sheet1!B1:B10"},
    {"range": "Sheet1!C1:C10"}
]

# Single API call for multiple ranges
result = service.spreadsheets().values().batchGet(
    spreadsheetId=spreadsheet_id,
    ranges=[req["range"] for req in batch_requests]
).execute()
```

### Caching
```python
# Cache frequently accessed data
import functools
import time

@functools.lru_cache(maxsize=128)
def cached_sheet_read(spreadsheet_id, range_name, timestamp):
    """Cache results for 5 minutes"""
    return read_sheet_data(spreadsheet_id, range_name)

# Use with 5-minute cache
timestamp = int(time.time() / 300)  # 5-minute buckets
data = cached_sheet_read(spreadsheet_id, range_name, timestamp)
```

## Dependencies

```bash
# Python packages
pip install google-auth google-auth-oauthlib google-auth-httplib2 google-api-python-client

# Optional for data processing
pip install pandas numpy

# System requirements
python3 --version  # Python 3.7+
```

## Configuration Files

### Service Account Example
```json
{
  "type": "service_account",
  "project_id": "your-project-id",
  "private_key_id": "key-id",
  "private_key": "-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----\n",
  "client_email": "service-account@your-project.iam.gserviceaccount.com",
  "client_id": "client-id",
  "auth_uri": "https://accounts.google.com/o/oauth2/auth",
  "token_uri": "https://oauth2.googleapis.com/token"
}
```

### OAuth2 Credentials Example
```json
{
  "installed": {
    "client_id": "your-client-id.apps.googleusercontent.com",
    "project_id": "your-project-id",
    "auth_uri": "https://accounts.google.com/o/oauth2/auth",
    "token_uri": "https://oauth2.googleapis.com/token",
    "client_secret": "your-client-secret",
    "redirect_uris": ["urn:ietf:wg:oauth:2.0:oob", "http://localhost"]
  }
}
```

## Troubleshooting

### Debug Mode
```bash
# Enable debug logging
export GOOGLE_SHEETS_DEBUG=true
python3 google_sheets_tools.py read "id" "A1:A10"
```

### Connection Issues
```bash
# Test API connectivity
curl -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  "https://sheets.googleapis.com/v4/spreadsheets/SPREADSHEET_ID"
```

### Quota Monitoring
```bash
# Check API usage in Google Cloud Console
gcloud logging read "resource.type=gce_instance AND logName=projects/PROJECT_ID/logs/sheets.googleapis.com"
```