#!/usr/bin/env python3
"""
Simple SSE MCP Server for Testing MagicTunnel's SSE Client

This is a basic SSE MCP server that implements a few test tools to verify
MagicTunnel's SSE client functionality.

Usage:
    python3 test-servers/simple_sse_mcp_server.py [--port PORT]

Requirements:
    pip install fastapi uvicorn mcp
"""

import asyncio
import json
import logging
import time
from datetime import datetime
from typing import Any, Dict, List, Optional

from fastapi import FastAPI
from fastapi.responses import StreamingResponse
from fastapi.middleware.cors import CORSMiddleware
import uvicorn

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

app = FastAPI(title="Simple SSE MCP Server", version="1.0.0")

# Enable CORS for local testing
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

class SimpleMcpServer:
    def __init__(self):
        self.tools = {
            "echo": {
                "name": "echo",
                "description": "Echo back the provided message",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Message to echo back"
                        }
                    },
                    "required": ["message"]
                }
            },
            "current_time": {
                "name": "current_time",
                "description": "Get the current server time",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            "add_numbers": {
                "name": "add_numbers",
                "description": "Add two numbers together",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "a": {
                            "type": "number",
                            "description": "First number"
                        },
                        "b": {
                            "type": "number",
                            "description": "Second number"
                        }
                    },
                    "required": ["a", "b"]
                }
            },
            "generate_sequence": {
                "name": "generate_sequence",
                "description": "Generate a sequence of numbers",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "start": {
                            "type": "integer",
                            "description": "Start number"
                        },
                        "end": {
                            "type": "integer",
                            "description": "End number"
                        }
                    },
                    "required": ["start", "end"]
                }
            }
        }
        
    def handle_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Handle incoming MCP requests"""
        try:
            method = request.get("method")
            params = request.get("params", {})
            request_id = request.get("id", "unknown")
            
            logger.info(f"Handling request: {method} (id: {request_id})")
            
            if method == "initialize":
                return {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {},
                            "resources": {},
                            "prompts": {}
                        },
                        "serverInfo": {
                            "name": "simple-sse-mcp-server",
                            "version": "1.0.0"
                        }
                    }
                }
                
            elif method == "tools/list":
                return {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "tools": list(self.tools.values())
                    }
                }
                
            elif method == "tools/call":
                tool_name = params.get("name")
                arguments = params.get("arguments", {})
                
                if tool_name not in self.tools:
                    return {
                        "jsonrpc": "2.0",
                        "id": request_id,
                        "error": {
                            "code": -32602,
                            "message": f"Unknown tool: {tool_name}"
                        }
                    }
                
                # Execute the tool
                result = self.execute_tool(tool_name, arguments)
                
                return {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": result
                            }
                        ]
                    }
                }
                
            else:
                return {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {
                        "code": -32601,
                        "message": f"Method not found: {method}"
                    }
                }
                
        except Exception as e:
            logger.error(f"Error handling request: {e}")
            return {
                "jsonrpc": "2.0",
                "id": request.get("id", "unknown"),
                "error": {
                    "code": -32603,
                    "message": f"Internal error: {str(e)}"
                }
            }
    
    def execute_tool(self, tool_name: str, arguments: Dict[str, Any]) -> str:
        """Execute a tool and return the result"""
        if tool_name == "echo":
            message = arguments.get("message", "")
            return f"Echo: {message}"
            
        elif tool_name == "current_time":
            return f"Current server time: {datetime.now().isoformat()}"
            
        elif tool_name == "add_numbers":
            a = arguments.get("a", 0)
            b = arguments.get("b", 0)
            result = a + b
            return f"{a} + {b} = {result}"
            
        elif tool_name == "generate_sequence":
            start = arguments.get("start", 0)
            end = arguments.get("end", 10)
            if end - start > 100:  # Prevent large sequences
                return "Error: Sequence too large (max 100 numbers)"
            sequence = list(range(start, end + 1))
            return f"Sequence from {start} to {end}: {sequence}"
            
        else:
            return f"Error: Unknown tool {tool_name}"

# Global MCP server instance
mcp_server = SimpleMcpServer()

@app.get("/")
async def root():
    """Health check endpoint"""
    return {
        "status": "healthy",
        "server": "simple-sse-mcp-server",
        "version": "1.0.0",
        "endpoints": {
            "sse": "/sse",
            "health": "/"
        },
        "tools": list(mcp_server.tools.keys())
    }

@app.get("/sse")
async def sse_endpoint():
    """SSE endpoint for MCP communication"""
    
    async def event_stream():
        """Generate SSE events for MCP communication"""
        logger.info("New SSE connection established")
        
        # Send initial connection event
        yield f"data: {json.dumps({'type': 'connection', 'status': 'connected', 'timestamp': time.time()})}\n\n"
        
        # Keep connection alive and handle incoming requests
        # Note: This is a simplified implementation
        # In a real implementation, you'd handle bidirectional communication
        
        try:
            # Send periodic heartbeat
            while True:
                await asyncio.sleep(30)  # Heartbeat every 30 seconds
                heartbeat = {
                    "type": "heartbeat",
                    "timestamp": time.time(),
                    "server_time": datetime.now().isoformat()
                }
                yield f"data: {json.dumps(heartbeat)}\n\n"
                
        except asyncio.CancelledError:
            logger.info("SSE connection closed")
            yield f"data: {json.dumps({'type': 'connection', 'status': 'closed', 'timestamp': time.time()})}\n\n"
        except Exception as e:
            logger.error(f"SSE stream error: {e}")
            error_event = {
                "type": "error",
                "message": str(e),
                "timestamp": time.time()
            }
            yield f"data: {json.dumps(error_event)}\n\n"
    
    return StreamingResponse(
        event_stream(), 
        media_type="text/plain",
        headers={
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Headers": "*",
        }
    )

@app.post("/mcp")
async def mcp_endpoint(request: Dict[str, Any]):
    """HTTP endpoint for MCP requests (alternative to SSE)"""
    logger.info(f"Received MCP request: {request}")
    response = mcp_server.handle_request(request)
    logger.info(f"Sending MCP response: {response}")
    return response

if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description="Simple SSE MCP Server")
    parser.add_argument("--port", type=int, default=8000, help="Port to run server on")
    parser.add_argument("--host", type=str, default="127.0.0.1", help="Host to bind to")
    parser.add_argument("--debug", action="store_true", help="Enable debug logging")
    
    args = parser.parse_args()
    
    if args.debug:
        logging.getLogger().setLevel(logging.DEBUG)
    
    logger.info(f"Starting Simple SSE MCP Server on {args.host}:{args.port}")
    logger.info("Available endpoints:")
    logger.info(f"  Health check: http://{args.host}:{args.port}/")
    logger.info(f"  SSE endpoint: http://{args.host}:{args.port}/sse")
    logger.info(f"  MCP endpoint: http://{args.host}:{args.port}/mcp")
    logger.info("Available tools: echo, current_time, add_numbers, generate_sequence")
    
    uvicorn.run(
        app, 
        host=args.host, 
        port=args.port,
        log_level="info" if not args.debug else "debug"
    )