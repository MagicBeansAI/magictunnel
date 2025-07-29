#!/usr/bin/env python3
"""
Google Sheets Tools for MagicTunnel with OAuth2 Support
Supports both OAuth2 (user authorization) and Service Account authentication
"""

import json
import sys
import os
import re
import pickle
from pathlib import Path
from google.auth.transport.requests import Request
from google.oauth2.credentials import Credentials
from google.oauth2 import service_account
from google_auth_oauthlib.flow import InstalledAppFlow
from googleapiclient.discovery import build

# Scopes required for Google Sheets and Drive access
SCOPES = [
    'https://www.googleapis.com/auth/spreadsheets',
    'https://www.googleapis.com/auth/drive'
]

def normalize_range(range_name):
    """Normalize Google Sheets range to handle common issues"""
    if not range_name:
        return "A:Z"  # Default to all data
    
    # Handle patterns like "Sheet1!A1:Z" (missing row number)
    pattern = r'^([^!]+!)([A-Z]+)(\d+):([A-Z]+)$'
    match = re.match(pattern, range_name)
    if match:
        sheet_part, start_col, start_row, end_col = match.groups()
        return f"{sheet_part}{start_col}:{end_col}"
    
    # Handle patterns like "A1:Z" (no sheet name)
    pattern = r'^([A-Z]+)(\d+):([A-Z]+)$'
    match = re.match(pattern, range_name)
    if match:
        start_col, start_row, end_col = match.groups()
        return f"{start_col}:{end_col}"
    
    # Handle single sheet name
    if '!' not in range_name and ':' not in range_name:
        return f"{range_name}!A:Z"
    
    return range_name

def get_oauth_credentials():
    """Get OAuth2 credentials with automatic browser authorization"""
    creds = None
    token_file = os.environ.get('GOOGLE_OAUTH_TOKEN_FILE', './token.pickle')
    credentials_file = os.environ.get('GOOGLE_OAUTH_CREDENTIALS_FILE', './credentials.json')
    
    # Load existing token
    if os.path.exists(token_file):
        with open(token_file, 'rb') as token:
            creds = pickle.load(token)
    
    # If there are no (valid) credentials available, let the user log in
    if not creds or not creds.valid:
        if creds and creds.expired and creds.refresh_token:
            try:
                creds.refresh(Request())
            except Exception as e:
                print(f"Error refreshing token: {e}", file=sys.stderr)
                creds = None
        
        if not creds:
            if not os.path.exists(credentials_file):
                raise Exception(f"OAuth credentials file not found: {credentials_file}")
            
            flow = InstalledAppFlow.from_client_secrets_file(credentials_file, SCOPES)
            creds = flow.run_local_server(port=0)
        
        # Save the credentials for the next run
        with open(token_file, 'wb') as token:
            pickle.dump(creds, token)
    
    return creds

def get_service_account_credentials():
    """Get Service Account credentials"""
    credentials_path = os.environ.get('GOOGLE_APPLICATION_CREDENTIALS')
    if not credentials_path or not os.path.exists(credentials_path):
        raise Exception("Service account credentials not found")
    
    credentials = service_account.Credentials.from_service_account_file(
        credentials_path, scopes=SCOPES
    )
    return credentials

def get_sheets_service():
    """Get Google Sheets service with automatic auth method detection"""
    # Try OAuth2 first if credentials.json exists
    oauth_creds_file = os.environ.get('GOOGLE_OAUTH_CREDENTIALS_FILE', './credentials.json')
    service_account_file = os.environ.get('GOOGLE_APPLICATION_CREDENTIALS')
    
    try:
        if os.path.exists(oauth_creds_file):
            print("Using OAuth2 authentication", file=sys.stderr)
            credentials = get_oauth_credentials()
        elif service_account_file and os.path.exists(service_account_file):
            print("Using Service Account authentication", file=sys.stderr)
            credentials = get_service_account_credentials()
        else:
            raise Exception("No authentication credentials found. Set either GOOGLE_OAUTH_CREDENTIALS_FILE or GOOGLE_APPLICATION_CREDENTIALS")
        
        return build('sheets', 'v4', credentials=credentials)
    except Exception as e:
        return {"error": f"Authentication failed: {str(e)}"}

def get_drive_service():
    """Get Google Drive service with automatic auth method detection"""
    oauth_creds_file = os.environ.get('GOOGLE_OAUTH_CREDENTIALS_FILE', './credentials.json')
    service_account_file = os.environ.get('GOOGLE_APPLICATION_CREDENTIALS')
    
    try:
        if os.path.exists(oauth_creds_file):
            credentials = get_oauth_credentials()
        elif service_account_file and os.path.exists(service_account_file):
            credentials = get_service_account_credentials()
        else:
            raise Exception("No authentication credentials found")
        
        return build('drive', 'v3', credentials=credentials)
    except Exception as e:
        return {"error": f"Authentication failed: {str(e)}"}

def read_spreadsheet(spreadsheet_id, range_name):
    """Read data from a Google Spreadsheet"""
    try:
        service = get_sheets_service()
        if isinstance(service, dict) and "error" in service:
            return service
        
        normalized_range = normalize_range(range_name)
        result = service.spreadsheets().values().get(
            spreadsheetId=spreadsheet_id,
            range=normalized_range
        ).execute()
        
        values = result.get('values', [])
        return {
            "success": True,
            "data": values,
            "range": result.get('range', normalized_range),
            "majorDimension": result.get('majorDimension', 'ROWS')
        }
    except Exception as e:
        return {"error": f"Failed to read spreadsheet: {str(e)}"}

def write_spreadsheet(spreadsheet_id, range_name, values):
    """Write data to a Google Spreadsheet"""
    try:
        service = get_sheets_service()
        if isinstance(service, dict) and "error" in service:
            return service
        
        normalized_range = normalize_range(range_name)
        body = {
            'values': values
        }
        
        result = service.spreadsheets().values().update(
            spreadsheetId=spreadsheet_id,
            range=normalized_range,
            valueInputOption='RAW',
            body=body
        ).execute()
        
        return {
            "success": True,
            "updatedCells": result.get('updatedCells', 0),
            "updatedRows": result.get('updatedRows', 0),
            "updatedColumns": result.get('updatedColumns', 0),
            "updatedRange": result.get('updatedRange', normalized_range)
        }
    except Exception as e:
        return {"error": f"Failed to write to spreadsheet: {str(e)}"}

def create_spreadsheet(title):
    """Create a new Google Spreadsheet"""
    try:
        service = get_sheets_service()
        if isinstance(service, dict) and "error" in service:
            return service
        
        spreadsheet = {
            'properties': {
                'title': title
            }
        }
        
        result = service.spreadsheets().create(body=spreadsheet).execute()
        
        return {
            "success": True,
            "spreadsheetId": result.get('spreadsheetId'),
            "spreadsheetUrl": result.get('spreadsheetUrl'),
            "title": title
        }
    except Exception as e:
        return {"error": f"Failed to create spreadsheet: {str(e)}"}

def list_sheets(spreadsheet_id):
    """List all sheets in a spreadsheet"""
    try:
        service = get_sheets_service()
        if isinstance(service, dict) and "error" in service:
            return service
        
        result = service.spreadsheets().get(spreadsheetId=spreadsheet_id).execute()
        sheets = result.get('sheets', [])
        
        sheet_list = []
        for sheet in sheets:
            properties = sheet.get('properties', {})
            sheet_list.append({
                'sheetId': properties.get('sheetId'),
                'title': properties.get('title'),
                'index': properties.get('index'),
                'sheetType': properties.get('sheetType', 'GRID')
            })
        
        return {
            "success": True,
            "sheets": sheet_list,
            "spreadsheetTitle": result.get('properties', {}).get('title', 'Unknown')
        }
    except Exception as e:
        return {"error": f"Failed to list sheets: {str(e)}"}

def list_spreadsheets():
    """List all accessible spreadsheets"""
    try:
        service = get_drive_service()
        if isinstance(service, dict) and "error" in service:
            return service
        
        results = service.files().list(
            q="mimeType='application/vnd.google-apps.spreadsheet'",
            fields="files(id,name,modifiedTime,webViewLink)"
        ).execute()
        
        files = results.get('files', [])
        
        return {
            "success": True,
            "spreadsheets": [
                {
                    "id": file['id'],
                    "name": file['name'],
                    "modifiedTime": file.get('modifiedTime'),
                    "webViewLink": file.get('webViewLink')
                }
                for file in files
            ]
        }
    except Exception as e:
        return {"error": f"Failed to list spreadsheets: {str(e)}"}

def main():
    """Command line interface"""
    if len(sys.argv) < 2:
        print("Usage: python google_sheets_oauth_tools.py <command> [args...]")
        print("Commands:")
        print("  read <spreadsheet_id> <range>")
        print("  write <spreadsheet_id> <range> <values_json>")
        print("  create <title>")
        print("  list_sheets <spreadsheet_id>")
        print("  list_spreadsheets")
        sys.exit(1)
    
    command = sys.argv[1]
    
    if command == "read":
        if len(sys.argv) != 4:
            print("Usage: read <spreadsheet_id> <range>")
            sys.exit(1)
        result = read_spreadsheet(sys.argv[2], sys.argv[3])
    
    elif command == "write":
        if len(sys.argv) != 5:
            print("Usage: write <spreadsheet_id> <range> <values_json>")
            sys.exit(1)
        try:
            values = json.loads(sys.argv[4])
        except json.JSONDecodeError:
            print("Error: values must be valid JSON")
            sys.exit(1)
        result = write_spreadsheet(sys.argv[2], sys.argv[3], values)
    
    elif command == "create":
        if len(sys.argv) != 3:
            print("Usage: create <title>")
            sys.exit(1)
        result = create_spreadsheet(sys.argv[2])
    
    elif command == "list_sheets":
        if len(sys.argv) != 3:
            print("Usage: list_sheets <spreadsheet_id>")
            sys.exit(1)
        result = list_sheets(sys.argv[2])
    
    elif command == "list_spreadsheets":
        result = list_spreadsheets()
    
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)
    
    print(json.dumps(result, indent=2))

if __name__ == "__main__":
    main()