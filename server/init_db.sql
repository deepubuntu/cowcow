-- Initialize CowCow Database Schema
-- This script is run automatically when the PostgreSQL container starts

-- Create database if it doesn't exist (handled by POSTGRES_DB env var)

-- Enable UUID extension for generating UUIDs
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(120) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    api_key VARCHAR(64) UNIQUE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    is_verified BOOLEAN DEFAULT FALSE
);

-- Create recordings table
CREATE TABLE IF NOT EXISTS recordings (
    id VARCHAR(36) PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lang VARCHAR(10) NOT NULL,
    prompt TEXT,
    qc_metrics JSONB NOT NULL,
    file_path TEXT NOT NULL,
    file_size BIGINT,
    duration_seconds REAL,
    status VARCHAR(20) DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    uploaded_at TIMESTAMP WITH TIME ZONE,
    CONSTRAINT recordings_status_check CHECK (status IN ('pending', 'uploading', 'completed', 'failed'))
);

-- Create tokens table for reward system
CREATE TABLE IF NOT EXISTS tokens (
    id VARCHAR(36) PRIMARY KEY DEFAULT uuid_generate_v4()::text,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount INTEGER NOT NULL,
    type VARCHAR(50) NOT NULL,
    description TEXT,
    recording_id VARCHAR(36) REFERENCES recordings(id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT tokens_type_check CHECK (type IN ('recording', 'bonus', 'penalty', 'withdrawal', 'referral'))
);

-- Create upload_queue table for managing uploads
CREATE TABLE IF NOT EXISTS upload_queue (
    recording_id VARCHAR(36) PRIMARY KEY REFERENCES recordings(id) ON DELETE CASCADE,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_attempt TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    priority INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create sessions table for JWT token management
CREATE TABLE IF NOT EXISTS sessions (
    id VARCHAR(36) PRIMARY KEY DEFAULT uuid_generate_v4()::text,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_accessed TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    ip_address INET,
    user_agent TEXT
);

-- Create quality_thresholds table for configurable QC settings
CREATE TABLE IF NOT EXISTS quality_thresholds (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL,
    min_snr_db REAL NOT NULL DEFAULT 20.0,
    max_clipping_pct REAL NOT NULL DEFAULT 1.0,
    min_vad_ratio REAL NOT NULL DEFAULT 80.0,
    min_duration_seconds REAL NOT NULL DEFAULT 1.0,
    max_duration_seconds REAL NOT NULL DEFAULT 300.0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_recordings_user_id ON recordings(user_id);
CREATE INDEX IF NOT EXISTS idx_recordings_lang ON recordings(lang);
CREATE INDEX IF NOT EXISTS idx_recordings_status ON recordings(status);
CREATE INDEX IF NOT EXISTS idx_recordings_created_at ON recordings(created_at);
CREATE INDEX IF NOT EXISTS idx_tokens_user_id ON tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_tokens_type ON tokens(type);
CREATE INDEX IF NOT EXISTS idx_tokens_created_at ON tokens(created_at);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_upload_queue_priority ON upload_queue(priority);
CREATE INDEX IF NOT EXISTS idx_upload_queue_created_at ON upload_queue(created_at);

-- Create a GIN index for JSONB QC metrics for efficient querying
CREATE INDEX IF NOT EXISTS idx_recordings_qc_metrics ON recordings USING GIN (qc_metrics);

-- Insert default quality thresholds
INSERT INTO quality_thresholds (name, min_snr_db, max_clipping_pct, min_vad_ratio) 
VALUES ('default', 20.0, 1.0, 80.0)
ON CONFLICT (name) DO NOTHING;

INSERT INTO quality_thresholds (name, min_snr_db, max_clipping_pct, min_vad_ratio) 
VALUES ('high_quality', 25.0, 0.5, 85.0)
ON CONFLICT (name) DO NOTHING;

INSERT INTO quality_thresholds (name, min_snr_db, max_clipping_pct, min_vad_ratio) 
VALUES ('low_resource', 15.0, 2.0, 70.0)
ON CONFLICT (name) DO NOTHING;

-- Create updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers for updated_at columns
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_quality_thresholds_updated_at BEFORE UPDATE ON quality_thresholds
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Create view for user statistics
CREATE OR REPLACE VIEW user_stats AS
SELECT 
    u.id,
    u.username,
    u.email,
    u.created_at as user_created_at,
    COUNT(r.id) as total_recordings,
    COUNT(CASE WHEN r.status = 'completed' THEN 1 END) as completed_recordings,
    COUNT(CASE WHEN r.status = 'pending' THEN 1 END) as pending_recordings,
    COALESCE(SUM(r.duration_seconds), 0) as total_duration_seconds,
    COALESCE(SUM(t.amount), 0) as total_tokens,
    COALESCE(AVG((r.qc_metrics->>'snr_db')::float), 0) as avg_snr_db,
    COALESCE(AVG((r.qc_metrics->>'clipping_pct')::float), 0) as avg_clipping_pct,
    COALESCE(AVG((r.qc_metrics->>'vad_ratio')::float), 0) as avg_vad_ratio
FROM users u
LEFT JOIN recordings r ON u.id = r.user_id
LEFT JOIN tokens t ON u.id = t.user_id AND t.amount > 0
GROUP BY u.id, u.username, u.email, u.created_at;

-- Create view for recording quality analysis
CREATE OR REPLACE VIEW recording_quality_stats AS
SELECT 
    lang,
    COUNT(*) as total_recordings,
    AVG((qc_metrics->>'snr_db')::float) as avg_snr_db,
    AVG((qc_metrics->>'clipping_pct')::float) as avg_clipping_pct,
    AVG((qc_metrics->>'vad_ratio')::float) as avg_vad_ratio,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY (qc_metrics->>'snr_db')::float) as median_snr_db,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY (qc_metrics->>'clipping_pct')::float) as median_clipping_pct,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY (qc_metrics->>'vad_ratio')::float) as median_vad_ratio
FROM recordings 
WHERE status = 'completed'
GROUP BY lang;

-- Grant permissions to cowcow_user
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO cowcow_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO cowcow_user;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO cowcow_user;

-- Insert a default admin user for testing (password: 'admin123')
-- This should be removed or changed in production
INSERT INTO users (username, email, password_hash, api_key) 
VALUES (
    'admin',
    'admin@cowcow.local',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBLK5QN7ZmJKhq',  -- bcrypt hash of 'admin123'
    'dev_api_key_' || encode(gen_random_bytes(32), 'hex')
) ON CONFLICT (username) DO NOTHING;

COMMIT;