# CowCow Docker Deployment Guide

This guide covers how to deploy CowCow using Docker for both development and production environments.

## Quick Start

### Prerequisites

- **Docker** (20.10+): [Install Docker](https://docs.docker.com/engine/install/)
- **Docker Compose** (2.0+): [Install Docker Compose](https://docs.docker.com/compose/install/)

### One-Click Development Setup

```bash
# Clone and navigate to the repository
git clone https://github.com/thabhelo/cowcow.git
cd cowcow

# Run the interactive setup script
./scripts/docker-setup.sh

# Or start development mode directly
./scripts/docker-setup.sh dev
```

This will:
- Set up environment variables
- Start PostgreSQL, Redis, and MinIO
- Launch the API server with hot reloading
- Provide admin interfaces for database and cache management

### Development Services

Once started, you'll have access to:

| Service | URL | Credentials |
|---------|-----|-------------|
| **API Server** | http://localhost:8000 | - |
| **API Documentation** | http://localhost:8000/docs | - |
| **MinIO Console** | http://localhost:9001 | minioadmin/minioadmin123 |
| **PgAdmin** | http://localhost:5050 | admin@cowcow.local/admin123 |
| **Redis Commander** | http://localhost:8081 | - |

## Environment Configuration

### Step 1: Copy Environment Template

```bash
cp env.example .env
```

### Step 2: Configure Essential Variables

```bash
# Required: Generate a secure JWT secret
JWT_SECRET=$(openssl rand -hex 32)

# For development: Use MinIO (automatically configured)
# For production: Configure Cloudflare R2
R2_ACCESS_KEY=your-r2-access-key
R2_SECRET_KEY=your-r2-secret-key
R2_ENDPOINT=https://your-account.r2.cloudflarestorage.com
R2_BUCKET=cowcow-recordings
```

## Development Mode

### Starting Development Environment

```bash
# Option 1: Using the setup script
./scripts/docker-setup.sh dev

# Option 2: Using Docker Compose directly
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up --build
```

### Development Features

- **Hot Reloading**: Code changes automatically restart the server
- **Debug Mode**: Detailed logging and error messages
- **Local Storage**: MinIO provides S3-compatible storage locally
- **Database Admin**: PgAdmin for database management
- **Redis Management**: Redis Commander for cache inspection

### Stopping Development Environment

```bash
# Using the setup script
./scripts/docker-setup.sh stop

# Using Docker Compose directly
docker-compose -f docker-compose.yml -f docker-compose.dev.yml down
```

## Production Mode

### Prerequisites for Production

1. **SSL Certificates**: Place your SSL certificates in `nginx/ssl/`:
   ```bash
   nginx/ssl/server.crt
   nginx/ssl/server.key
   ```

2. **Environment Variables**: Configure production values in `.env`:
   ```bash
   DEBUG=false
   JWT_SECRET=your-production-jwt-secret
   R2_ACCESS_KEY=your-production-r2-key
   R2_SECRET_KEY=your-production-r2-secret
   ```

### Starting Production Environment

```bash
# Using the setup script
./scripts/docker-setup.sh prod

# Using Docker Compose directly
docker-compose -f docker-compose.yml -f docker-compose.prod.yml up --build -d
```

### Production Features

- **NGINX Reverse Proxy**: Load balancing and SSL termination
- **Multi-container API**: Horizontal scaling with 2+ API instances
- **Production Database**: Optimized PostgreSQL with backups
- **Monitoring**: Prometheus metrics and Grafana dashboards
- **Logging**: Centralized log aggregation with Loki
- **Security**: Rate limiting, security headers, and SSL enforcement

### Production Services

| Service | URL | Purpose |
|---------|-----|---------|
| **API Server** | https://localhost | Main application |
| **API Documentation** | https://localhost/docs | API reference |
| **Prometheus** | http://localhost:9090 | Metrics collection |
| **Grafana** | http://localhost:3000 | Monitoring dashboards |

## Docker Compose Files Overview

### Base Configuration (`docker-compose.yml`)
- PostgreSQL database
- Redis cache
- API server
- MinIO (development profile)
- NGINX (production profile)

### Development Override (`docker-compose.dev.yml`)
- Enables hot reloading
- Adds development tools (PgAdmin, Redis Commander)
- Uses MinIO for local storage
- Mounts source code for live editing

### Production Override (`docker-compose.prod.yml`)
- Optimized for production workloads
- Enables monitoring stack
- Adds backup services
- Configures resource limits
- Enables SSL and security features

## Common Operations

### Viewing Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f api

# Using the setup script
./scripts/docker-setup.sh logs
```

### Service Status

```bash
# Using Docker Compose
docker-compose ps

# Using the setup script
./scripts/docker-setup.sh status
```

### Database Access

```bash
# Connect to PostgreSQL directly
docker-compose exec postgres psql -U cowcow_user -d cowcow

# Or use PgAdmin at http://localhost:5050 (development mode)
```

### Scaling Services

```bash
# Scale API service to 3 instances
docker-compose up --scale api=3
```

### Data Backup

```bash
# Backup database
docker-compose exec postgres pg_dump -U cowcow_user cowcow > backup.sql

# Backup uploads (if using local storage)
docker run --rm -v cowcow_uploads_data:/data -v $(pwd):/backup alpine tar czf /backup/uploads-backup.tar.gz -C /data .
```

### Data Restore

```bash
# Restore database
docker-compose exec -T postgres psql -U cowcow_user cowcow < backup.sql

# Restore uploads
docker run --rm -v cowcow_uploads_data:/data -v $(pwd):/backup alpine tar xzf /backup/uploads-backup.tar.gz -C /data
```

## Troubleshooting

### Common Issues

#### Port Conflicts
If ports are already in use, update the port mappings in `docker-compose.yml`:
```yaml
ports:
  - "8001:8000"  # Change host port from 8000 to 8001
```

#### Database Connection Issues
Check if PostgreSQL is ready:
```bash
docker-compose exec postgres pg_isready -U cowcow_user
```

#### Storage Issues with MinIO
Create the bucket manually:
```bash
# Access MinIO container
docker-compose exec minio mc mb /data/cowcow-recordings
```

### Reset Everything

```bash
# Stop all services and remove data
./scripts/docker-setup.sh cleanup

# Or manually
docker-compose down -v --rmi all
docker system prune -a
```

### Performance Tuning

#### For Development
- Increase Docker Desktop memory allocation (4GB+)
- Use Docker volume mounts instead of bind mounts for better performance on macOS/Windows

#### For Production
- Tune PostgreSQL configuration in `postgres` service environment
- Adjust worker processes in NGINX configuration
- Scale API service based on load:
  ```bash
  docker-compose up --scale api=4
  ```

## Security Considerations

### Development Security
- Change default passwords in `env.example`
- Don't expose development ports in production
- Use separate databases for dev/prod

### Production Security
- Use strong, unique passwords for all services
- Configure proper SSL certificates
- Enable firewall rules to restrict access
- Regularly update Docker images
- Monitor logs for suspicious activity

### Secrets Management
For production, use Docker secrets:
```bash
echo "your-password" | docker secret create postgres_password -
```

## Monitoring and Logging

### Metrics Collection
Prometheus collects metrics from:
- API server performance
- Database connections
- Request rates and errors
- System resources

### Log Aggregation
Logs are collected by Loki and can be viewed in Grafana:
- Application logs
- Access logs
- Error logs
- System logs

### Alerts
Configure Grafana alerts for:
- High error rates
- Database connection issues
- Storage space warnings
- Performance degradation

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Deploy to Production
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Deploy
        run: |
          ./scripts/docker-setup.sh prod
```

### Health Checks
All services include health checks for monitoring:
- API: `/health` endpoint
- Database: PostgreSQL connection test
- Redis: PING command
- NGINX: Configuration validation

## Next Steps

After setting up Docker deployment:

1. **Configure Monitoring**: Set up Grafana dashboards
2. **Set Up Backups**: Configure automated database backups
3. **Domain Setup**: Configure proper domain names and SSL certificates
4. **Scale Planning**: Plan for horizontal scaling based on usage
5. **Security Review**: Conduct security audit of the deployment

For more information, see:
- [Configuration Guide](configuration.md)
- [Architecture Overview](architecture.md)
- [API Documentation](http://localhost:8000/docs)