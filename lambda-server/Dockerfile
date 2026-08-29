# Lambda Execution Server - Dockerfile
#
# Multi-stage build for production deployment
# Reference: §7.3 of agent-native-os-architecture.md ("Lambda base images")

# =============================================================================
# Stage 1: Builder
# =============================================================================
FROM python:3.12-slim as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files
COPY pyproject.toml ./

# Install Python dependencies
RUN pip install --no-cache-dir --upgrade pip && \
    pip install --no-cache-dir --prefix=/install .

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM python:3.12-slim as runtime

# Security: Run as non-root user
RUN groupadd -r lambda && \
    useradd -r -g lambda -d /app -s /sbin/nologin lambda

# Install runtime dependencies only
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy installed packages from builder
COPY --from=builder /install /usr/local

# Create directories
RUN mkdir -p /app /tmp/lambda-sockets && \
    chown -R lambda:lambda /app /tmp/lambda-sockets

WORKDIR /app

# Copy application code
COPY --chown=lambda:lambda . .

# Environment variables
ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    LAMBDA_HOST=0.0.0.0 \
    LAMBDA_PORT=8080 \
    LAMBDA_LOG_LEVEL=INFO

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Switch to non-root user
USER lambda

# Run the server
CMD ["python", "-m", "http_server"]
