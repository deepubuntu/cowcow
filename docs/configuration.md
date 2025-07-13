# Configuration Guide

Complete configuration reference for the Cowcow speech data collection system.

## Overview

Cowcow uses a two-tier configuration system:
- **Server Configuration**: Environment variables in `server/.env`
- **CLI Configuration**: TOML file at `~/.cowcow/config.toml`

## Server Configuration

### Basic Setup (Development)

Create `server/.env` with minimal configuration:

```bash
# JWT Authentication
JWT_SECRET=dev-secret-key-change-in-production-this-is-not-secure
JWT_ALGORITHM=HS256
JWT_EXPIRE_MINUTES=1440

# Database (SQLite for development)
DATABASE_URL=sqlite:///./cowcow_server.db

# Optional - Cloud Storage (can use dummy values for local testing)
R2_ACCESS_KEY=test-access-key
R2_SECRET_KEY=test-secret-key
R2_ENDPOINT=https://test.cloudflareapi.com
R2_BUCKET=test-bucket
```

### Production Configuration

For production environments:

```bash
# JWT Authentication (REQUIRED - Generate secure key)
JWT_SECRET=your-secure-64-character-jwt-secret-key-here
JWT_ALGORITHM=HS256
JWT_EXPIRE_MINUTES=1440

# Database (PostgreSQL recommended for production)
DATABASE_URL=postgresql://username:password@localhost:5432/cowcow_production

# Cloudflare R2 Storage (Optional)
R2_ACCESS_KEY=your-cloudflare-r2-access-key
R2_SECRET_KEY=your-cloudflare-r2-secret-key  
R2_ENDPOINT=https://your-account-id.r2.cloudflarestorage.com
R2_BUCKET=cowcow-recordings

# Server Settings
HOST=0.0.0.0
PORT=8000
```

### Generating Secure Keys

**JWT Secret Key:**
```bash
# Method 1: Python
python3 -c "import secrets; print(secrets.token_hex(32))"

# Method 2: OpenSSL
openssl rand -hex 32

# Method 3: Online (use with caution)
# Visit: https://www.allkeysgenerator.com/Random/Security-Encryption-Key-Generator.aspx
```

### Environment Variable Reference

| Variable | Default | Description |
|----------|---------|-------------|
| `JWT_SECRET` | **Required** | Secret key for JWT token signing |
| `JWT_ALGORITHM` | `HS256` | JWT signing algorithm |
| `JWT_EXPIRE_MINUTES` | `1440` | Token expiration (24 hours) |
| `DATABASE_URL` | `sqlite:///./cowcow_server.db` | Database connection string |
| `R2_ACCESS_KEY` | `test-access-key` | Cloudflare R2 access key |
| `R2_SECRET_KEY` | `test-secret-key` | Cloudflare R2 secret key |
| `R2_ENDPOINT` | `https://test.cloudflareapi.com` | R2 endpoint URL |
| `R2_BUCKET` | `test-bucket` | R2 bucket name |

## CLI Configuration

### Configuration File Location

The CLI configuration is automatically created at:
- **macOS/Linux**: `~/.cowcow/config.toml`
- **Windows**: `%USERPROFILE%\.cowcow\config.toml`

### Default Configuration

```toml
[api]
endpoint = "http://localhost:8000"
timeout_secs = 30

[storage]
data_dir = "/Users/username/.cowcow"  # Auto-detected
auto_upload = false

[audio]
sample_rate = 48000
channels = 1
min_snr_db = 20.0
max_clipping_pct = 1.0
min_vad_ratio = 80.0

[upload]
max_retries = 3
retry_delay_secs = 2
chunk_size = 1048576
```

### Configuration Sections

#### API Settings (`[api]`)

```toml
[api]
endpoint = "http://localhost:8000"    # Server URL
timeout_secs = 30                     # Request timeout
```

**Production example:**
```toml
[api]
endpoint = "https://api.cowcow.example.com"
timeout_secs = 60
```

#### Storage Settings (`[storage]`)

```toml
[storage]
data_dir = "/Users/username/.cowcow"  # Data directory
auto_upload = false                   # Upload after recording
```

- `data_dir`: Where recordings and database are stored
- `auto_upload`: If `true`, uploads immediately after recording

#### Audio Settings (`[audio]`)

```toml
[audio]
sample_rate = 16000      # Audio sample rate (Hz)
channels = 1             # Number of audio channels
min_snr_db = 20.0       # Minimum SNR for upload
max_clipping_pct = 1.0  # Maximum clipping percentage
min_vad_ratio = 80.0    # Minimum voice activity ratio
```

**Quality Control Thresholds:**
- `min_snr_db`: Recordings below this SNR are rejected (default: 20.0 dB)
- `max_clipping_pct`: Recordings above this clipping are rejected (default: 1.0%)
- `min_vad_ratio`: Recordings below this voice activity are rejected (default: 80.0%)

**Sample Rate Options:**
- `16000`: Standard quality (default, ~32KB per 10s)
- `48000`: High quality (~96KB per 10s)
- `8000`: Minimum quality (~16KB per 10s)

#### Upload Settings (`[upload]`)

```toml
[upload]
max_retries = 3         # Maximum upload attempts
retry_delay_secs = 2    # Delay between retries
chunk_size = 1048576    # Upload chunk size (1MB)
```

## Intelligent Silence Detection

The silence detection system is configured through code constants (in `cowcow_cli/src/main.rs`):

```rust
// Silence detection parameters
let silence_threshold_secs = 5.0; // Stop after 5 seconds of silence
let vad_threshold = 0.01;         // VAD ratio threshold (1%)
let rms_threshold = 0.005;        // RMS level threshold
```

### Silence Detection Behavior

1. **Voice Activity Detection**: Uses both WebRTC VAD and RMS level
2. **Silence Timer**: Starts when no voice activity detected
3. **Timer Reset**: Resets when voice activity resumes
4. **Auto-stop**: Stops recording after 5 seconds of continuous silence

### Tuning Sensitivity

To modify silence detection sensitivity, edit the thresholds in the source code:

- **Less sensitive** (ignores more background noise): Increase `rms_threshold` to `0.01`
- **More sensitive** (detects quieter speech): Decrease `rms_threshold` to `0.003`

## Managing Configuration

### View Current Configuration

```bash
./target/release/cowcow_cli config show
```

### Update Configuration

```bash
# Reset to defaults
./target/release/cowcow_cli config reset

# Note: config set command is not yet implemented
# To change individual values, edit ~/.cowcow/config.toml manually
```

### Configuration Examples

#### High Quality Recording
```toml
[audio]
sample_rate = 48000
min_snr_db = 25.0
max_clipping_pct = 0.5
min_vad_ratio = 90.0
```

#### Mobile/Bandwidth Optimized
```toml
[audio]
sample_rate = 16000
min_snr_db = 15.0
max_clipping_pct = 2.0
min_vad_ratio = 60.0

[upload]
chunk_size = 524288  # 512KB chunks
```

#### Auto-upload Everything
```toml
[storage]
auto_upload = true

[audio]
min_snr_db = 0.0      # Accept any quality
max_clipping_pct = 100.0
min_vad_ratio = 0.0
```

## Authentication Configuration

### API Keys

API keys are automatically generated and stored in `~/.cowcow/credentials.json`:

```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "api_key": "88f8a81f5e817ef7e2e50230...",
  "username": "testuser",
  "expires_at": 1752477796
}
```

### Authentication Flow

1. **Register**: `./target/release/cowcow_cli auth register`
2. **Login**: `./target/release/cowcow_cli auth login` 
3. **Auto-renewal**: Tokens refresh automatically
4. **Manual refresh**: Re-login if authentication fails

### Multi-device Setup

To use the same account on multiple devices:

1. Login on first device
2. Copy `~/.cowcow/credentials.json` to other devices
3. Or register separate accounts and manage via server

## Database Configuration

### Local Database (SQLite)

Default location: `~/.cowcow/cowcow.db`

**Schema:**
```sql
-- Recordings table
CREATE TABLE recordings (
    id TEXT PRIMARY KEY,
    lang TEXT NOT NULL,
    prompt TEXT,
    qc_metrics TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    uploaded_at INTEGER,
    wav_path TEXT NOT NULL
);

-- Upload queue
CREATE TABLE upload_queue (
    recording_id TEXT PRIMARY KEY,
    attempts INTEGER NOT NULL,
    last_attempt INTEGER,
    FOREIGN KEY (recording_id) REFERENCES recordings(id)
);
```

### Server Database

**Development (SQLite):**
```bash
DATABASE_URL=sqlite:///./cowcow_server.db
```

**Production (PostgreSQL):**
```bash
DATABASE_URL=postgresql://username:password@localhost:5432/cowcow
```

## Cloud Storage Configuration

### Cloudflare R2 Setup

1. **Create R2 Bucket**:
   - Login to Cloudflare Dashboard
   - Navigate to R2 Object Storage
   - Create new bucket

2. **Generate API Tokens**:
   - Go to R2 → Manage R2 API Tokens
   - Create token with Object Read/Write permissions
   - Copy Access Key ID and Secret Access Key

3. **Update Configuration**:
   ```bash
   # Add to server/.env
   R2_ACCESS_KEY=your-access-key-from-cloudflare
   R2_SECRET_KEY=your-secret-key-from-cloudflare
   R2_ENDPOINT=https://your-account-id.r2.cloudflarestorage.com
   R2_BUCKET=your-bucket-name
   ```

### Alternative Storage

The system supports any S3-compatible storage:

```bash
# AWS S3
R2_ENDPOINT=https://s3.amazonaws.com
R2_BUCKET=your-s3-bucket

# MinIO
R2_ENDPOINT=https://your-minio-server.com
R2_BUCKET=cowcow-recordings

# DigitalOcean Spaces
R2_ENDPOINT=https://nyc3.digitaloceanspaces.com
R2_BUCKET=your-space-name
```

## Security Considerations

### Development vs Production

**Development:**
- Use default JWT secret (not secure)
- SQLite database
- Local file storage
- HTTP endpoints

**Production:**
- Generate secure JWT secret (64+ characters)
- PostgreSQL database with SSL
- Cloud storage with encryption
- HTTPS endpoints only
- Regular key rotation

### Best Practices

1. **Never commit secrets** to version control
2. **Use environment variables** for all secrets
3. **Rotate API keys** regularly
4. **Use HTTPS** in production
5. **Backup databases** regularly
6. **Monitor access logs**

## Troubleshooting Configuration

### Common Issues

#### 1. "Invalid JWT secret"
```bash
# Generate new secret
openssl rand -hex 32
# Update server/.env with new JWT_SECRET
```

#### 2. "Database connection failed"
```bash
# Check database URL format
# SQLite: sqlite:///./path/to/database.db
# PostgreSQL: postgresql://user:pass@host:port/dbname
```

#### 3. "Configuration file not found"
```bash
# Reset configuration
rm ~/.cowcow/config.toml
./target/release/cowcow_cli config show  # Recreates with defaults
```

#### 4. "Permission denied"
```bash
# Fix permissions
chmod 600 ~/.cowcow/credentials.json
chmod 644 ~/.cowcow/config.toml
```

### Validation

Test your configuration:

```bash
# Check CLI config
./target/release/cowcow_cli config show

# Check server health
curl http://localhost:8000/health

# Test authentication
./target/release/cowcow_cli auth status

# Full system check
./target/release/cowcow_cli doctor
```

---

For more configuration examples, see the [Setup Guide](SETUP.md) or [Architecture Documentation](architecture.md). 