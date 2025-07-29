#!/usr/bin/env python3
"""
Google Sheets Tools for MagicTunnel
Reliable Google Sheets integration using service account authentication
"""

import json
import sys
import os
import re
from google.oauth2 import service_account
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
    # Pattern: sheet!col1num:col (e.g., Sheet1!A1:Z)
    pattern = r'^([^!]+!)([A-Z]+)(\d+):([A-Z]+)$'
    match = re.match(pattern, range_name)
    if match:
        sheet_part, start_col, start_row, end_col = match.groups()
        # Convert to full columns range
        return f"{sheet_part}{start_col}:{end_col}"
    
    # Handle patterns like "A1:Z" (no sheet name, missing end row)
    pattern = r'^([A-Z]+)(\d+):([A-Z]+)$'
    match = re.match(pattern, range_name)
    if match:
        start_col, start_row, end_col = match.groups()
        # Convert to full columns range
        return f"{start_col}:{end_col}"
    
    # Handle patterns like "Sheet1!A:Z" (already correct full column ranges)
    if ':' in range_name and ('!' not in range_name or re.match(r'^[^!]+![A-Z]+:[A-Z]+$', range_name)):
        return range_name
    
    # Handle complete cell ranges (already correct) like "Sheet1!A1:Z10"
    if re.match(r'^[^!]+![A-Z]+\d+:[A-Z]+\d+$', range_name):
        return range_name
    
    # Handle single sheet names or cells - default to all data
    if '!' not in range_name:
        # If it looks like just a sheet name, append range
        return f"{range_name}!A:Z" if range_name else "A:Z"
    
    # Handle sheet!cell patterns (single cell) - expand to reasonable range
    if re.match(r'^[^!]+![A-Z]+\d+$', range_name):
        # Single cell reference - expand to a reasonable range around it
        parts = range_name.split('!')
        if len(parts) == 2:
            sheet_name, cell = parts
            # Extract column and expand to full column
            col_match = re.match(r'^([A-Z]+)\d+$', cell)
            if col_match:
                col = col_match.group(1)
                return f"{sheet_name}!{col}:{col}"
    
    # Default fallback - return as is
    return range_name

def get_sheets_service():
    """Initialize Google Sheets service with service account credentials"""
    credentials_path = os.environ.get('GOOGLE_APPLICATION_CREDENTIALS')
    if not credentials_path:
        raise Exception("GOOGLE_APPLICATION_CREDENTIALS environment variable not set")
    
    credentials = service_account.Credentials.from_service_account_file(
        credentials_path, scopes=SCOPES
    )
    
    return build('sheets', 'v4', credentials=credentials)

def get_drive_service():
    """Initialize Google Drive service with service account credentials"""
    credentials_path = os.environ.get('GOOGLE_APPLICATION_CREDENTIALS')
    if not credentials_path:
        raise Exception("GOOGLE_APPLICATION_CREDENTIALS environment variable not set")
    
    credentials = service_account.Credentials.from_service_account_file(
        credentials_path, scopes=SCOPES
    )
    
    return build('drive', 'v3', credentials=credentials)

def read_spreadsheet(spreadsheet_id, range_name):
    """Read data from a Google Sheets spreadsheet"""
    try:
        service = get_sheets_service()
        
        # Clean up range name - remove extra escaping
        range_name = range_name.replace('\\!', '!')
        
        # Normalize incomplete ranges
        range_name = normalize_range(range_name)
        
        result = service.spreadsheets().values().get(
            spreadsheetId=spreadsheet_id,
            range=range_name
        ).execute()
        
        values = result.get('values', [])
        
        return {
            "success": True,
            "data": values,
            "range": range_name,
            "spreadsheet_id": spreadsheet_id,
            "rows_returned": len(values)
        }
        
    except Exception as e:
        return {
            "success": False,
            "error": str(e),
            "spreadsheet_id": spreadsheet_id,
            "range": range_name
        }

def write_spreadsheet(spreadsheet_id, range_name, values):
    """Write data to a Google Sheets spreadsheet"""
    try:
        service = get_sheets_service()
        
        # Parse values if it's a JSON string
        if isinstance(values, str):
            values = json.loads(values)
        
        body = {
            'values': values
        }
        
        result = service.spreadsheets().values().update(
            spreadsheetId=spreadsheet_id,
            range=range_name,
            valueInputOption='RAW',
            body=body
        ).execute()
        
        return {
            "success": True,
            "updated_cells": result.get('updatedCells', 0),
            "updated_range": result.get('updatedRange', ''),
            "spreadsheet_id": spreadsheet_id
        }
        
    except Exception as e:
        return {
            "success": False,
            "error": str(e),
            "spreadsheet_id": spreadsheet_id,
            "range": range_name
        }

def create_spreadsheet(title):
    """Create a new Google Sheets spreadsheet"""
    try:
        service = get_sheets_service()
        
        spreadsheet = {
            'properties': {
                'title': title
            }
        }
        
        result = service.spreadsheets().create(body=spreadsheet).execute()
        
        return {
            "success": True,
            "spreadsheet_id": result.get('spreadsheetId'),
            "title": title,
            "url": f"https://docs.google.com/spreadsheets/d/{result.get('spreadsheetId')}"
        }
        
    except Exception as e:
        return {
            "success": False,
            "error": str(e),
            "title": title
        }

def list_sheets(spreadsheet_id):
    """List all sheets in a spreadsheet"""
    try:
        service = get_sheets_service()
        
        result = service.spreadsheets().get(spreadsheetId=spreadsheet_id).execute()
        
        sheets = []
        for sheet in result.get('sheets', []):
            properties = sheet.get('properties', {})
            sheets.append({
                'sheet_id': properties.get('sheetId'),
                'title': properties.get('title'),
                'index': properties.get('index'),
                'sheet_type': properties.get('sheetType', 'GRID'),
                'grid_properties': properties.get('gridProperties', {})
            })
        
        return {
            "success": True,
            "spreadsheet_id": spreadsheet_id,
            "spreadsheet_title": result.get('properties', {}).get('title', ''),
            "sheets": sheets,
            "total_sheets": len(sheets)
        }
        
    except Exception as e:
        return {
            "success": False,
            "error": str(e),
            "spreadsheet_id": spreadsheet_id
        }

def list_spreadsheets():
    """List all spreadsheets accessible to the service account"""
    try:
        drive_service = get_drive_service()
        
        # Query for Google Sheets files
        query = "mimeType='application/vnd.google-apps.spreadsheet'"
        
        results = drive_service.files().list(
            q=query,
            fields="files(id, name, createdTime, modifiedTime, owners)",
            orderBy="modifiedTime desc"
        ).execute()
        
        files = results.get('files', [])
        
        spreadsheets = []
        for file in files:
            spreadsheets.append({
                'id': file.get('id'),
                'name': file.get('name'),
                'created_time': file.get('createdTime'),
                'modified_time': file.get('modifiedTime'),
                'url': f"https://docs.google.com/spreadsheets/d/{file.get('id')}",
                'owners': [owner.get('displayName', 'Unknown') for owner in file.get('owners', [])]
            })
        
        return {
            "success": True,
            "spreadsheets": spreadsheets,
            "total_count": len(spreadsheets)
        }
        
    except Exception as e:
        return {
            "success": False,
            "error": str(e)
        }

def main():
    if len(sys.argv) < 2:
        print(json.dumps({
            "success": False,
            "error": "Usage: python google_sheets_tools.py <command> [args...]"
        }))
        sys.exit(1)
    
    command = sys.argv[1]
    
    try:
        if command == "read":
            if len(sys.argv) != 4:
                raise ValueError("Usage: read <spreadsheet_id> <range>")
            result = read_spreadsheet(sys.argv[2], sys.argv[3])
            
        elif command == "write":
            if len(sys.argv) != 5:
                raise ValueError("Usage: write <spreadsheet_id> <range> <values_json>")
            result = write_spreadsheet(sys.argv[2], sys.argv[3], sys.argv[4])
            
        elif command == "create":
            if len(sys.argv) != 3:
                raise ValueError("Usage: create <title>")
            result = create_spreadsheet(sys.argv[2])
            
        elif command == "list_sheets":
            if len(sys.argv) != 3:
                raise ValueError("Usage: list_sheets <spreadsheet_id>")
            result = list_sheets(sys.argv[2])
            
        elif command == "list_spreadsheets":
            result = list_spreadsheets()
            
        else:
            result = {
                "success": False,
                "error": f"Unknown command: {command}. Available: read, write, create, list_sheets, list_spreadsheets"
            }
            
    except Exception as e:
        result = {
            "success": False,
            "error": str(e),
            "command": command
        }
    
    print(json.dumps(result, indent=2))

if __name__ == "__main__":
    main()