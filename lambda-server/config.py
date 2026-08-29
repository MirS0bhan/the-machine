"""
Configuration management for the Lambda Execution Server.

This module provides centralized configuration management with support
for environment variables and configuration files.

Version: 0.1.0
"""

import os
import logging
from dataclasses import dataclass, field
from typing import Optional

logger = logging.getLogger(__name__)


@dataclass
class ServerConfig:
    """
    Configuration for the Lambda Server.
    
    Attributes:
        host: Host to bind the server to
        port: Port to listen on
        debug: Enable debug mode
        log_level: Logging level
        socket_dir: Directory for IPC sockets
        max_warm_per_function: Maximum warm processes per function
        warm_timeout_seconds: How long warm processes can idle
        heartbeat_interval: Expected heartbeat interval
        max_total_warm: Maximum total warm processes
        lease_ttl_seconds: Default lease time-to-live
    """
    host: str = "0.0.0.0"
    port: int = 8080
    debug: bool = False
    log_level: str = "INFO"
    socket_dir: str = "/tmp/lambda-sockets"
    max_warm_per_function: int = 2
    warm_timeout_seconds: float = 300.0
    heartbeat_interval: float = 30.0
    max_total_warm: int = 50
    lease_ttl_seconds: float = 300.0
    
    @classmethod
    def from_env(cls) -> "ServerConfig":
        """
        Create configuration from environment variables.
        
        Environment Variables:
            LAMBDA_HOST: Host to bind to (default: 0.0.0.0)
            LAMBDA_PORT: Port to listen on (default: 8080)
            LAMBDA_DEBUG: Enable debug mode (default: false)
            LAMBDA_LOG_LEVEL: Logging level (default: INFO)
            LAMBDA_SOCKET_DIR: Directory for IPC sockets
            LAMBDA_MAX_WARM_PER_FUNCTION: Maximum warm processes per function
            LAMBDA_WARM_TIMEOUT_SECONDS: Warm process timeout
            LAMBDA_HEARTBEAT_INTERVAL: Heartbeat interval
            LAMBDA_MAX_TOTAL_WARM: Maximum total warm processes
            LAMBDA_LEASE_TTL_SECONDS: Lease TTL
            
        Returns:
            ServerConfig instance
        """
        return cls(
            host=os.environ.get("LAMBDA_HOST", "0.0.0.0"),
            port=int(os.environ.get("LAMBDA_PORT", "8080")),
            debug=os.environ.get("LAMBDA_DEBUG", "false").lower() == "true",
            log_level=os.environ.get("LAMBDA_LOG_LEVEL", "INFO"),
            socket_dir=os.environ.get("LAMBDA_SOCKET_DIR", "/tmp/lambda-sockets"),
            max_warm_per_function=int(os.environ.get("LAMBDA_MAX_WARM_PER_FUNCTION", "2")),
            warm_timeout_seconds=float(os.environ.get("LAMBDA_WARM_TIMEOUT_SECONDS", "300")),
            heartbeat_interval=float(os.environ.get("LAMBDA_HEARTBEAT_INTERVAL", "30")),
            max_total_warm=int(os.environ.get("LAMBDA_MAX_TOTAL_WARM", "50")),
            lease_ttl_seconds=float(os.environ.get("LAMBDA_LEASE_TTL_SECONDS", "300")),
        )
    
    def setup_logging(self) -> None:
        """Configure logging based on settings."""
        log_format = "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
        logging.basicConfig(
            level=getattr(logging, self.log_level.upper()),
            format=log_format,
        )
        logger.info(f"Logging configured: level={self.log_level}")
