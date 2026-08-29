"""
HTTP Server for the Lambda Execution Server.

This module provides an HTTP API for the Lambda Server, implementing
the MCP control interface over HTTP/JSON.

Version: 0.1.0
"""

import json
import logging
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Any, Dict
from urllib.parse import urlparse, parse_qs

from server import LambdaServer
from config import ServerConfig

logger = logging.getLogger(__name__)


class LambdaRequestHandler(BaseHTTPRequestHandler):
    """
    HTTP request handler for the Lambda Server.
    
    Endpoints:
        POST /mcp/{tool_name} - Handle MCP tool calls
        GET /health - Health check
        GET /stats - Server statistics
        GET /tools - List available tools
    """
    
    server: LambdaServer
    
    def do_GET(self) -> None:
        """Handle GET requests."""
        parsed = urlparse(self.path)
        path = parsed.path
        
        if path == "/health":
            self._handle_health()
        elif path == "/stats":
            self._handle_stats()
        elif path == "/tools":
            self._handle_tools()
        else:
            self._send_error(404, "Not found")
    
    def do_POST(self) -> None:
        """Handle POST requests."""
        parsed = urlparse(self.path)
        path = parsed.path
        
        if path.startswith("/mcp/"):
            tool_name = path[5:]  # Remove "/mcp/" prefix
            self._handle_mcp_tool(tool_name)
        else:
            self._send_error(404, "Not found")
    
    def _handle_health(self) -> None:
        """Handle health check endpoint."""
        result = self.server.app.health_check()
        self._send_json(200, result)
    
    def _handle_stats(self) -> None:
        """Handle stats endpoint."""
        result = self.server.app.handle_mcp_tool("lambda.get_stats", {})
        self._send_json(200, result)
    
    def _handle_tools(self) -> None:
        """Handle tools listing endpoint."""
        tools = self.server.app.get_tools()
        self._send_json(200, {"tools": tools})
    
    def _handle_mcp_tool(self, tool_name: str) -> None:
        """Handle MCP tool call."""
        try:
            content_length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(content_length) if content_length > 0 else b""
            
            if body:
                args = json.loads(body.decode("utf-8"))
            else:
                args = {}
            
            result = self.server.app.handle_mcp_tool(tool_name, args)
            self._send_json(200, result)
        except json.JSONDecodeError as e:
            self._send_error(400, f"Invalid JSON: {e}")
        except Exception as e:
            logger.error(f"Error handling {tool_name}: {e}", exc_info=True)
            self._send_error(500, str(e))
    
    def _send_json(self, status: int, data: Dict[str, Any]) -> None:
        """Send JSON response."""
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(data).encode("utf-8"))
    
    def _send_error(self, status: int, message: str) -> None:
        """Send error response."""
        self._send_json(status, {"error": message})
    
    def log_message(self, format: str, *args: Any) -> None:
        """Log HTTP requests."""
        logger.info(f"{self.address_string()} - {format % args}")


def create_http_server(config: Optional[ServerConfig] = None) -> HTTPServer:
    """
    Create and configure the HTTP server.
    
    Args:
        config: Server configuration
        
    Returns:
        Configured HTTPServer instance
    """
    if config is None:
        config = ServerConfig.from_env()
    
    server_instance = LambdaServer()
    
    server = HTTPServer(
        (config.host, config.port),
        LambdaRequestHandler,
    )
    server.app = server_instance  # type: ignore
    
    logger.info(f"HTTP server created on {config.host}:{config.port}")
    return server


def run_server(config: Optional[ServerConfig] = None) -> None:
    """
    Run the HTTP server.
    
    Args:
        config: Server configuration
    """
    if config is None:
        config = ServerConfig.from_env()
    
    config.setup_logging()
    
    server = create_http_server(config)
    
    logger.info(f"Starting Lambda Server on {config.host}:{config.port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        logger.info("Shutting down server...")
        server.shutdown()


if __name__ == "__main__":
    run_server()
