# Multi-stage build for production optimization
FROM python:3.11-slim as base

# Set environment variables
ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PIP_NO_CACHE_DIR=1 \
    PIP_DISABLE_PIP_VERSION_CHECK=1

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    gcc \
    g++ \
    make \
    pkg-config \
    libffi-dev \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app user for security
RUN groupadd -r appuser && useradd -r -g appuser appuser

# Set work directory
WORKDIR /app

# Development stage
FROM base as development

# Install development dependencies
RUN apt-get update && apt-get install -y \
    git \
    vim \
    && rm -rf /var/lib/apt/lists/*

# Copy requirements first for better caching
COPY requirements.txt .
COPY requirements-dev.txt .

# Install Python dependencies
RUN pip install --upgrade pip && \
    pip install -r requirements-dev.txt

# Copy application code
COPY . .

# Change ownership to app user
RUN chown -R appuser:appuser /app

# Switch to app user
USER appuser

# Expose ports
EXPOSE 8000 50051

# Run development server with hot reload
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000", "--reload"]

# Production stage
FROM base as production

# Copy requirements
COPY requirements.txt .

# Install production dependencies only
RUN pip install --upgrade pip && \
    pip install -r requirements.txt && \
    pip install gunicorn[gthread]

# Copy application code
COPY . .

# Create uploads directory
RUN mkdir -p /app/uploads

# Change ownership to app user
RUN chown -R appuser:appuser /app

# Switch to app user
USER appuser

# Expose ports
EXPOSE 8000 50051

# Health check
HEALTHCHECK --interval=30s --timeout=30s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Run production server
CMD ["gunicorn", "main:app", "-w", "4", "-k", "uvicorn.workers.UvicornWorker", "--bind", "0.0.0.0:8000", "--timeout", "120"]