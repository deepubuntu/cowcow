# Cowcow Testing Guide 🧪

Complete testing guide with practical examples to verify all Cowcow features are working correctly.

## Quick Health Check

Before running detailed tests, verify your system is properly configured:

```bash
# 1. Check system health
./target/release/cowcow_cli doctor

# Expected output:
# 🔍 System Health Check
#   Audio device: ✅
#   Storage directory: ✅ 
#   Database: ✅
#   Server connection: ✅
#   Authentication: ✅
```

If any items show ❌, refer to the [Setup Guide](SETUP.md) for troubleshooting.

## Test Scenarios

### 1. Authentication System

#### Test User Registration

```bash
# Register a new test user
./target/release/cowcow_cli auth register
```

**Example session:**
```
Username: testuser
Email: test@example.com
Password: [hidden]
✅ Registration successful! You can now login.
```

#### Test Login

```bash
# Login with your account
./target/release/cowcow_cli auth login
```

**Example session:**
```
Username: testuser
Password: [hidden]
✅ Login successful!
```

#### Verify Authentication Status

```bash
# Check current auth status
./target/release/cowcow_cli auth status
```

**Expected output:**
```
✅ Authenticated
  Username: testuser
  Expires: 2025-07-14 08:30:15
```

### 2. Basic Recording Tests

#### Test 1: Simple Recording with Auto-stop

```bash
# Record with intelligent silence detection
./target/release/cowcow_cli record --lang en
```

**What to expect:**
1. **Countdown**: 3-second preparation time
2. **Recording starts**: Shows real-time metrics
3. **Voice detection**: Should show "VOICE DETECTED" when speaking
4. **Silence tracking**: Shows silence duration when quiet
5. **Auto-stop**: Stops after 5 seconds of silence

**Example output:**
```
Get ready to speak...
Starting in 3...
Starting in 2...
Starting in 1...
🎙️  RECORDING NOW!
⠁ Recording... SNR: 24.4 dB | Clipping: 0.0% | VAD: 0.0% | RMS: 0.0023 | Silence: 5.0s

Silence detected for 5.0s
  Recording... Recording complete!

Recording Quality Metrics:
  SNR: 24.4 dB
  Clipping: 0.0%
  Voice Activity: 0.0%
```

#### Test 2: Fixed Duration Recording

```bash
# Record for exactly 10 seconds
./target/release/cowcow_cli record --lang en --duration 10
```

**Expected behavior:**
- Records for exactly 10 seconds regardless of silence
- Shows countdown and real-time metrics
- Stops with "Duration reached" message

#### Test 3: Recording with Prompt

```bash
# Record with a text prompt
./target/release/cowcow_cli record --lang en --duration 5 --prompt "The quick brown fox jumps over the lazy dog"
```

**Expected behavior:**
- Shows the prompt text before recording
- Waits for Enter key press
- Records normally after prompt display

### 3. Intelligent Silence Detection Tests

#### Test 1: Voice Activity Reset

```bash
# Start recording and test silence timer reset
./target/release/cowcow_cli record --lang en
```

**Testing procedure:**
1. Start recording (countdown begins)
2. Stay silent for 3-4 seconds (watch silence counter)
3. Speak loudly or clap (silence timer should reset)
4. Stay silent again (timer restarts from 0)
5. Wait 5 seconds of silence (auto-stops)

**Expected behavior:**
- Silence counter increases when quiet
- Shows "VOICE DETECTED" when speaking
- Silence timer resets to 0 when voice activity detected
- Auto-stops after 5 continuous seconds of silence

#### Test 2: Different Voice Levels

Test the RMS threshold sensitivity:

```bash
# Multiple recordings to test sensitivity
./target/release/cowcow_cli record --lang test_whisper  # Try whispering
./target/release/cowcow_cli record --lang test_normal   # Normal speaking
./target/release/cowcow_cli record --lang test_loud     # Loud speaking
```

**Expected RMS levels:**
- **Background noise**: ~0.001-0.003 (should not trigger voice detection)
- **Whisper**: ~0.003-0.006 (may or may not trigger, depending on threshold)
- **Normal speech**: ~0.005-0.020 (should trigger voice detection)
- **Loud speech**: ~0.020+ (should definitely trigger)

### 4. Quality Control Tests

#### Test 1: High Quality Recording

Create a high-quality recording:

```bash
# Record in a quiet environment, speak clearly
./target/release/cowcow_cli record --lang quality_test --duration 5
```

**Target metrics:**
- **SNR**: > 20 dB
- **Clipping**: < 1%
- **File size**: ~240KB for 5 seconds

#### Test 2: Poor Quality Recording

Test quality thresholds:

```bash
# Record in noisy environment or speak too softly
./target/release/cowcow_cli record --lang poor_quality --duration 5
```

**Expected behavior:**
- Lower SNR values
- Upload may be rejected if below thresholds

#### Test 3: Quality Configuration

Adjust quality thresholds:

```bash
# Edit ~/.cowcow/config.toml to lower quality requirements:
# [audio]
# min_snr_db = 10.0
# min_vad_ratio = 50.0

# Test with relaxed requirements
./target/release/cowcow_cli record --lang relaxed_quality --duration 5
```

### 5. Upload System Tests

#### Test 1: Basic Upload

```bash
# Upload quality recordings only
./target/release/cowcow_cli upload
```

**Expected behavior:**
- Shows pending recordings count
- Uploads recordings that meet quality thresholds
- Skips recordings below thresholds
- Shows upload progress and results

**Example output:**
```
Found 3 pending recordings
Uploading recording: abc123... (240KB)
✅ Upload successful: 2 tokens awarded
Upload summary: 1 successful, 2 skipped
```

#### Test 2: Force Upload

```bash
# Upload all recordings regardless of quality
./target/release/cowcow_cli upload --force
```

**Expected behavior:**
- Uploads ALL pending recordings
- Ignores quality thresholds
- Shows upload status for each file

#### Test 3: Upload with Network Issues

Test upload resilience:

```bash
# Stop the server temporarily
# In server terminal: Ctrl+C

# Try upload (should fail gracefully)
./target/release/cowcow_cli upload

# Restart server
cd server && uvicorn main:app --reload --host 0.0.0.0 --port 8000

# Retry upload (should succeed)
./target/release/cowcow_cli upload
```

### 6. Multi-language Tests

Test different language codes:

```bash
# Test various language codes
./target/release/cowcow_cli record --lang en    # English
./target/release/cowcow_cli record --lang sw    # Swahili
./target/release/cowcow_cli record --lang fr    # French
./target/release/cowcow_cli record --lang zu    # Zulu
./target/release/cowcow_cli record --lang test  # Custom code
```

**Expected behavior:**
- Creates separate directories for each language
- Accepts any language code
- Organizes recordings by language

### 7. Configuration Tests

#### Test 1: View Configuration

```bash
# Display current configuration
./target/release/cowcow_cli config show
```

#### Test 2: Modify Configuration

```bash
# Note: config set command is not yet implemented
# To change settings, edit ~/.cowcow/config.toml manually

# Verify changes
./target/release/cowcow_cli config show
```

#### Test 3: Reset Configuration

```bash
# Reset to defaults
./target/release/cowcow_cli config reset

# Verify reset
./target/release/cowcow_cli config show
```

### 8. Statistics and Monitoring

#### Test Recording Statistics

```bash
# View recording statistics
./target/release/cowcow_cli stats
```

**Expected output:**
```
📊 Recording Statistics
  Total recordings: 15
  Uploaded: 12
  Pending: 3
```

### 9. Server API Tests

#### Test 1: Health Check

```bash
# Test server health endpoint
curl http://localhost:8000/health
```

**Expected response:**
```json
{"status":"healthy","timestamp":"2025-07-13T08:30:00.123456"}
```

#### Test 2: API Documentation

Open in browser:
- **Swagger UI**: http://localhost:8000/docs
- **ReDoc**: http://localhost:8000/redoc

#### Test 3: Authentication API

```bash
# Get API key from credentials
API_KEY=$(cat ~/.cowcow/credentials.json | python3 -c "import sys, json; print(json.load(sys.stdin)['api_key'])")

# Test authenticated endpoint
curl -H "X-API-Key: $API_KEY" http://localhost:8000/recordings
```

### 10. File System Tests

#### Test 1: Recording File Structure

```bash
# Check recording directory structure
ls -la ~/.cowcow/recordings/

# Check specific language directory
ls -la ~/.cowcow/recordings/en/
```

**Expected structure:**
```
~/.cowcow/
├── config.toml
├── credentials.json
├── cowcow.db
└── recordings/
    ├── en/
    │   ├── recording1.wav
    │   └── recording2.wav
    ├── sw/
    └── fr/
```

#### Test 2: File Properties

```bash
# Check a recording file
python3 -c "
import wave
import os

# Replace with actual filename
filename = '~/.cowcow/recordings/en/your-recording-id.wav'
filepath = os.path.expanduser(filename)

if os.path.exists(filepath):
    with wave.open(filepath, 'rb') as w:
        print(f'Sample Rate: {w.getframerate()} Hz')
        print(f'Channels: {w.getnchannels()}')
        print(f'Duration: {w.getnframes() / w.getframerate():.2f} seconds')
        print(f'File Size: {os.path.getsize(filepath)} bytes')
"
```

### 11. Error Handling Tests

#### Test 1: No Microphone

```bash
# Test with no audio device (simulation)
# This should fail gracefully with clear error message
```

#### Test 2: Server Offline

```bash
# Stop server, try recording and upload
# Should work for recording, fail gracefully for upload
```

#### Test 3: Invalid Configuration

```bash
# Edit ~/.cowcow/config.toml to use invalid server:
# [api] 
# endpoint = "http://invalid-server.com"
./target/release/cowcow_cli upload

# Edit back to valid endpoint:
# [api]
# endpoint = "http://localhost:8000"
```

## Performance Tests

### Test 1: Recording Performance

Time the recording process:

```bash
# Time a recording
time ./target/release/cowcow_cli record --lang perf_test --duration 10
```

**Performance targets:**
- Audio processing: < 200ms latency
- File creation: Immediate
- Quality metrics: Real-time calculation

### Test 2: Upload Performance

Test upload speed:

```bash
# Create several recordings
for i in {1..5}; do
    ./target/release/cowcow_cli record --lang perf_test_$i --duration 3
done

# Time batch upload
time ./target/release/cowcow_cli upload --force
```

### Test 3: Storage Efficiency

Check file sizes:

```bash
# 10-second recording at 48kHz should be ~960KB
./target/release/cowcow_cli record --lang storage_test --duration 10

# Check actual file size
ls -lh ~/.cowcow/recordings/storage_test/
```

## Regression Tests

### Complete Workflow Test

Test the entire end-to-end workflow:

```bash
#!/bin/bash
# Complete workflow test script

echo "1. Testing authentication..."
./target/release/cowcow_cli auth status || exit 1

echo "2. Testing recording..."
./target/release/cowcow_cli record --lang regression_test --duration 3 || exit 1

echo "3. Testing upload..."
./target/release/cowcow_cli upload --force || exit 1

echo "4. Testing stats..."
./target/release/cowcow_cli stats || exit 1

echo "5. Testing server health..."
curl -f http://localhost:8000/health || exit 1

echo "✅ All tests passed!"
```

## Expected Test Results

### Successful Test Indicators

- ✅ **Authentication**: Login/register works, tokens valid
- ✅ **Recording**: Files created with correct duration and format
- ✅ **Silence Detection**: Auto-stops after 5s, resets on voice activity
- ✅ **Quality Metrics**: SNR > 15dB, minimal clipping, appropriate file sizes
- ✅ **Upload**: Successful file transfer to server
- ✅ **Server**: Health check returns 200, API endpoints respond
- ✅ **Configuration**: Settings persist and can be modified

### Common Test Failures

- ❌ **"No audio device"**: Check microphone permissions
- ❌ **"Server connection failed"**: Ensure server is running on port 8000
- ❌ **"Authentication failed"**: Re-login or check credentials
- ❌ **"Build failed"**: Check Rust installation and dependencies
- ❌ **"Upload rejected"**: Check quality thresholds or use `--force`

## Automated Testing

Create a test script for regular validation:

```bash
#!/bin/bash
# save as test_cowcow.sh

set -e  # Exit on any error

echo "🧪 Running Cowcow Test Suite..."

# Test 1: System Health
echo "Testing system health..."
./target/release/cowcow_cli doctor > /dev/null || (echo "❌ Health check failed" && exit 1)

# Test 2: Recording
echo "Testing recording..."
./target/release/cowcow_cli record --lang auto_test --duration 2 > /dev/null || (echo "❌ Recording failed" && exit 1)

# Test 3: Upload
echo "Testing upload..."
./target/release/cowcow_cli upload --force > /dev/null || (echo "❌ Upload failed" && exit 1)

# Test 4: Server API
echo "Testing server API..."
curl -f -s http://localhost:8000/health > /dev/null || (echo "❌ Server API failed" && exit 1)

echo "✅ All tests passed! Cowcow is working correctly."
```

Make it executable and run:

```bash
chmod +x test_cowcow.sh
./test_cowcow.sh
```

---

**Need help with a specific test?** Check the [Setup Guide](SETUP.md) or [Configuration Guide](configuration.md) for detailed troubleshooting steps. 