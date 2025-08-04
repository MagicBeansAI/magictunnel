# Google Sheets Integration for MagicTunnel

## Overview

This document explains why we implemented a **custom Google Sheets integration** instead of using existing MCP packages, and how to use it.

## Why Custom Integration?

### Problems with Existing MCP Packages

During development, we attempted to integrate several existing Google Sheets MCP packages but encountered fundamental issues:

1. **[xing5/mcp-google-sheets](https://github.com/xing5/mcp-google-sheets)**
   - **Error**: `ValueError: a coroutine was expected, got None`
   - **Issue**: Fundamental coding errors in async/await handling
   - **Status**: Non-functional

2. **[mcp-gdrive](https://github.com/modelcontextprotocol/servers/tree/main/src/gdrive)**
   - **Error**: Module import failures and dependency issues
   - **Issue**: Broken package structure and missing dependencies
   - **Status**: Non-functional

3. **Authentication Mismatch**
   - Most existing MCP packages expect **OAuth authentication**
   - Our setup uses **Google Service Account credentials**
   - **Solution**: Custom implementation with service account support

### Benefits of Custom Solution

✅ **Reliable**: No dependency on broken third-party packages  
✅ **Service Account Support**: Works with our existing authentication setup  
✅ **Robust Error Handling**: Proper range normalization and error recovery  
✅ **MagicTunnel Integration**: Seamless subprocess routing with parameter substitution  
✅ **Special Character Support**: Handles sheet names with apostrophes like `SO_Daily_July'25`  

## Implementation Details

### Architecture

```
MagicTunnel Discovery → Parameter Extraction → Subprocess Routing → Python Script → Google APIs
```

### Components

1. **Tool Definitions** (`capabilities/google/google_sheets.yaml`)
   - 5 Google Sheets operations: read, write, create, list_sheets, list_spreadsheets
   - Subprocess routing configuration
   - Parameter validation schemas

2. **Python Script** (`google_sheets_tools.py`)
   - Service account authentication
   - Google Sheets API v4 and Drive API v3 integration
   - Range normalization (handles malformed ranges like `Sheet1!A1:Z` → `Sheet1!A:Z`)
   - JSON response formatting

3. **Service Account Authentication**
   - Uses `GOOGLE_APPLICATION_CREDENTIALS` environment variable
   - Supports both Google Sheets and Drive APIs
   - Secure credential management (credentials file in `.gitignore`)

### Key Features

#### 1. Intelligent Range Normalization
```python
# Handles common range issues automatically
"Sheet1!A1:Z" → "Sheet1!A:Z"        # Missing end row
"A1:Z" → "A:Z"                      # No sheet name
"Sheet1" → "Sheet1!A:Z"             # Default range
```

#### 2. Smart Tool Discovery Integration
```bash
# Natural language request
curl -X POST http://localhost:3001/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"name": "smart_tool_discovery", "arguments": {"request": "read from google sheets https://docs.google.com/spreadsheets/d/1U3Z25eyfu1mm0KyZTScSRpvSaPZK-A6j1-CvC3q0p7c/ sheet SO_Daily_July'\''25"}}'
```

#### 3. Direct Tool Access
```bash
# Direct tool call
curl -X POST http://localhost:3001/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"name": "google_sheets_read", "arguments": {"spreadsheet_id": "1U3Z25eyfu1mm0KyZTScSRpvSaPZK-A6j1-CvC3q0p7c", "range": "SO_Daily_July'\''25!A1:E10"}}'
```

## Available Tools

| Tool | Description | Required Parameters |
|------|-------------|-------------------|
| `google_sheets_read` | Read data from spreadsheet | `spreadsheet_id`, `range` |
| `google_sheets_write` | Write data to spreadsheet | `spreadsheet_id`, `range`, `values` |
| `google_sheets_create` | Create new spreadsheet | `title` |
| `google_sheets_list_sheets` | List sheets in spreadsheet | `spreadsheet_id` |
| `google_sheets_list_spreadsheets` | List all accessible spreadsheets | none |

## Setup Instructions

### 1. Service Account Setup
1. Create a Google Cloud service account
2. Enable Google Sheets API and Drive API
3. Download credentials JSON file
4. Place credentials file in project root
5. Add credentials filename to `.gitignore`

### 2. Environment Configuration
```bash
# Set in .env.development or environment
GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account.json"
```

### 3. Python Dependencies
```bash
# Install required packages
pip install google-auth google-auth-oauthlib google-auth-httplib2 google-api-python-client
```

## Usage Examples

### Reading Spreadsheet Data
```json
{
  "name": "google_sheets_read",
  "arguments": {
    "spreadsheet_id": "1U3Z25eyfu1mm0KyZTScSRpvSaPZK-A6j1-CvC3q0p7c",
    "range": "SO_Daily_July'25!A1:E10"
  }
}
```

### Finding Available Sheets
```json
{
  "name": "google_sheets_list_sheets",
  "arguments": {
    "spreadsheet_id": "1U3Z25eyfu1mm0KyZTScSRpvSaPZK-A6j1-CvC3q0p7c"
  }
}
```

### Creating New Spreadsheet
```json
{
  "name": "google_sheets_create",
  "arguments": {
    "title": "My New Spreadsheet"
  }
}
```

## Troubleshooting

### Common Issues

1. **Authentication Errors**
   - Verify `GOOGLE_APPLICATION_CREDENTIALS` is set correctly
   - Ensure service account has access to the spreadsheet
   - Check that Google Sheets API and Drive API are enabled

2. **Range Parsing Errors**
   - Use `list_sheets` first to find correct sheet names
   - The system automatically normalizes common range issues
   - For special characters, ensure proper JSON escaping

3. **Permission Errors**
   - Service account must have appropriate permissions on the spreadsheet
   - For newly created spreadsheets, they're owned by the service account
   - To access existing spreadsheets, share them with the service account email

### Debugging
```bash
# Test authentication directly
GOOGLE_APPLICATION_CREDENTIALS="./ak-gdrive-magictunnel.json" python3 google_sheets_tools.py list_spreadsheets

# Test specific operations
GOOGLE_APPLICATION_CREDENTIALS="./ak-gdrive-magictunnel.json" python3 google_sheets_tools.py read "SPREADSHEET_ID" "Sheet1!A1:C10"
```

## Security Notes

- ⚠️ **Service account credentials contain private keys** - never commit to version control
- ✅ **Credentials are in `.gitignore`** - they won't be accidentally committed
- ✅ **Use environment variables** for credential paths in production
- ✅ **Principle of least privilege** - only grant necessary API access

## Future Enhancements

- [ ] Batch operations for large datasets
- [ ] Real-time change notifications
- [ ] Advanced formatting and styling options
- [ ] Integration with Google Workspace APIs
- [ ] Caching layer for frequently accessed data

## API Reference

For detailed parameter schemas and response formats, see:
- Tool definitions: `capabilities/google/google_sheets.yaml`
- Implementation: `google_sheets_tools.py`
- MagicTunnel docs: [Smart Discovery Guide](CLAUDE.md)