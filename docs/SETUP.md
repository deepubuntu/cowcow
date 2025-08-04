# Cowcow Setup Guide 🚀

Complete step-by-step guide to get Cowcow running on your system.

## Prerequisites

### Required Software

1. **Rust (1.70+)**
   ```bash
   # Install Rust via rustup
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   
   # Verify installation
   rustc --version
   cargo --version
   ```

2. **Python (3.8+)**
   ```bash
   # Check if Python is installed
   python3 --version
   
   # Install if missing (macOS)
   brew install python3
   
   # Install if missing (Ubuntu/Debian)
   sudo apt update && sudo apt install python3 python3-pip
   ```

3. **Audio System**
   - **macOS**: Built-in audio system (works out of the box)
   - **Linux**: ALSA/PulseAudio
     ```bash
     # Ubuntu/Debian
     sudo apt install libasound2-dev
     
     # Fedora
     sudo dnf install alsa-lib-devel
     ```
   - **Windows**: Windows Audio Session API (WASAPI)

### Optional Dependencies

- **PostgreSQL** (for production): [Download PostgreSQL](https://postgresql.org/download)
- **Git**: For version control

## Installation Steps

### 1. Clone the Repository

```bash
git clone https://github.com/deepubuntu/cowcow.git
cd cowcow
```

### 2. Build the CLI Tool

```bash
# Build in release mode (recommended)
cargo build --release

# The binary will be at: ./target/release/cowcow_cli
# Alternatively, for development:
# cargo build  (binary at ./target/debug/cowcow_cli)
```

**Troubleshooting Build Issues:**

- **Missing dependencies**: Install system audio libraries (see Prerequisites)
- **Rust version**: Ensure Rust 1.70+ with `rustup update`
- **Permission errors**: Don't use `sudo` with Rust builds

### 3. Setup the Server

```bash
cd server

# Install Python dependencies
pip3 install -r requirements.txt

# Verify installation
python3 -c "import fastapi, uvicorn, sqlx; print('Dependencies installed successfully')"
```

### 4. Start the Server

```bash
# From the server directory
uvicorn main:app --reload --host 0.0.0.0 --port 8000
```

**Expected output:**
```
INFO:     Uvicorn running on http://0.0.0.0:8000 (Press CTRL+C to quit)
INFO:     Started reloader process [12345]
INFO:     Started server process [12346]
INFO:     Waiting for application startup.
INFO:     Application startup complete.
```

**Test the server:**
```bash
curl http://localhost:8000/health
# Expected: {"status":"healthy","timestamp":"..."}
```

### 5. First Time Setup

#### Register a User Account

```bash
./target/release/cowcow_cli auth register
```

**Example session:**
```
Username: testuser
Email: test@example.com
Password: [hidden]
✅ Registration successful! You can now login.
```

#### Login

```bash
./target/release/cowcow_cli auth login
```

**Example session:**
```
Username: testuser
Password: [hidden]
✅ Login successful!
```

#### Verify Setup

```bash
# Check system health
./target/release/cowcow_cli doctor
```

**Expected output:**
```
🔍 System Health Check
  Audio device: ✅
  Storage directory: ✅
  Database: ✅
  Server connection: ✅
  Authentication: ✅
```

## Basic Usage Examples

### 1. Simple Recording (with auto-stop)

```bash
./target/release/cowcow_cli record --lang en
```

**What happens:**
1. 3-second countdown
2. Recording starts
3. Auto-stops after 5 seconds of silence
4. Shows quality metrics
5. Saves to `~/.cowcow/recordings/en/`

### 2. Fixed Duration Recording

```bash
./target/release/cowcow_cli record --lang sw --duration 10
```

### 3. Recording with Prompt

```bash
./target/release/cowcow_cli record --lang fr --prompt "Bonjour, comment allez-vous?"
```

### 4. Upload Recordings

```bash
# Upload quality recordings only
./target/release/cowcow_cli upload

# Upload all recordings (ignore quality thresholds)
./target/release/cowcow_cli upload --force
```

### 5. Check Statistics

```bash
./target/release/cowcow_cli stats
```

**Example output:**
```
📊 Recording Statistics
  Total recordings: 15
  Uploaded: 12
  Pending: 3
```

## Configuration

### CLI Configuration

Located at `~/.cowcow/config.toml` (created automatically):

```toml
[api]
endpoint = "http://localhost:8000"
timeout_secs = 30

[storage]
data_dir = "/Users/username/.cowcow"
auto_upload = false

[audio]
sample_rate = 16000
channels = 1
min_snr_db = 20.0
max_clipping_pct = 1.0
min_vad_ratio = 80.0

[upload]
max_retries = 3
retry_delay_secs = 2
chunk_size = 1048576
```

### Server Configuration

For development, you can use the provided example files:

```bash
# Copy example environment files
cp .env.example .env
cp server/.env.example server/.env

# Edit with your values (optional for development)
nano server/.env
```

The default settings work fine for development. For production, update the values in `.env` files:

```bash
# Production settings (update in server/.env)
JWT_SECRET=your-secure-64-character-jwt-secret-key-here
DATABASE_URL=postgresql://user:pass@localhost:5432/cowcow
R2_ACCESS_KEY=your-cloudflare-r2-key
R2_SECRET_KEY=your-cloudflare-r2-secret
R2_ENDPOINT=https://account.r2.cloudflarestorage.com
R2_BUCKET=cowcow-recordings
```

## Advanced Features

### 1. Quality Control Thresholds

Adjust recording quality requirements:

```bash
# Edit ~/.cowcow/config.toml manually:
# audio.min_snr_db = 15.0
# audio.min_vad_ratio = 60.0
```

### 2. Auto-upload

Enable automatic upload after recording:

```bash
# Edit ~/.cowcow/config.toml manually:
# [storage]
# auto_upload = true
```

### 3. Custom Server Endpoint

Point to a different server:

```bash
# Edit ~/.cowcow/config.toml manually:
# [api]
# endpoint = "https://your-server.com"
```

## Troubleshooting

### Common Issues

#### 1. "No audio device found"

**macOS:**
- Check microphone permissions in System Preferences > Security & Privacy > Microphone
- Ensure your application terminal has microphone access

**Linux:**
```bash
# Check audio devices
arecord -l

# Test microphone
arecord -d 5 test.wav && aplay test.wav

# Install missing packages
sudo apt install alsa-utils pulseaudio
```

#### 2. "Server connection failed"

```bash
# Check if server is running
curl http://localhost:8000/health

# If not running, start server:
cd server && uvicorn main:app --reload --host 0.0.0.0 --port 8000

# Check for port conflicts
lsof -i :8000
```

#### 3. "Authentication failed"

```bash
# Check auth status
./target/release/cowcow_cli auth status

# Re-login if needed
./target/release/cowcow_cli auth logout
./target/release/cowcow_cli auth login
```

#### 4. Build fails with "linker errors"

**macOS:**
```bash
# Install Xcode command line tools
xcode-select --install
```

**Linux:**
```bash
# Install build essentials
sudo apt install build-essential

# Install missing system libraries
sudo apt install libasound2-dev pkg-config
```

#### 5. "Permission denied" errors

```bash
# Fix ownership of .cowcow directory
sudo chown -R $USER:$USER ~/.cowcow

# Don't use sudo with cargo
cargo clean && cargo build --release
```

### Performance Issues

#### 1. High CPU usage during recording

- Lower sample rate: `./target/release/cowcow_cli config set audio.sample_rate 16000`
- Increase buffer size (requires rebuild with modified code)

#### 2. Large file sizes

- Current: ~96KB per 10 seconds (48kHz, 16-bit mono)
- Reduce sample rate for smaller files (trades quality for size)

#### 3. Slow uploads

- Check network connection
- Increase retry delay: `./target/release/cowcow_cli config set upload.retry_delay_secs 5`

### Getting Help

1. **Check system health**: `./target/release/cowcow_cli doctor`
2. **View logs**: Server logs appear in terminal where uvicorn is running
3. **Reset configuration**: Delete `~/.cowcow/config.toml` (will recreate with defaults)
4. **Fresh start**: 
   ```bash
   rm -rf ~/.cowcow
   ./target/release/cowcow_cli auth register
   ```

## Production Deployment

### 1. Database Setup

For production, use PostgreSQL:

```bash
# Install PostgreSQL
# Ubuntu: sudo apt install postgresql postgresql-contrib
# macOS: brew install postgresql

# Create database
sudo -u postgres createdb cowcow_production

# Update server/.env
echo "DATABASE_URL=postgresql://postgres:password@localhost/cowcow_production" >> server/.env
```

### 2. Cloud Storage (Optional)

Setup Cloudflare R2 for file storage:

```bash
# Add to server/.env
R2_ACCESS_KEY=your-access-key
R2_SECRET_KEY=your-secret-key  
R2_ENDPOINT=https://account-id.r2.cloudflarestorage.com
R2_BUCKET=cowcow-prod
```

### 3. Production Server

```bash
# Install production WSGI server
pip install gunicorn

# Start production server
gunicorn main:app -w 4 -k uvicorn.workers.UvicornWorker --bind 0.0.0.0:8000
```

### 4. HTTPS Setup

Use a reverse proxy like Nginx with SSL certificates:

```nginx
server {
    listen 443 ssl;
    server_name your-domain.com;
    
    ssl_certificate /path/to/certificate.crt;
    ssl_certificate_key /path/to/private.key;
    
    location / {
        proxy_pass http://localhost:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Next Steps

1. **Try recording**: `./target/release/cowcow_cli record --lang en`
2. **Explore commands**: `./target/release/cowcow_cli --help`
3. **Read documentation**: Check `docs/` directory for detailed guides
4. **Join community**: Contribute to the project on GitHub

---

**Need more help?** Check the [Configuration Guide](configuration.md) or [Architecture Documentation](architecture.md). 