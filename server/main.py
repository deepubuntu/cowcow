import asyncio
import os
from datetime import datetime, timedelta
from typing import Optional

import boto3
from fastapi import FastAPI, HTTPException, Depends, status, Request, Form, File, UploadFile
from fastapi.middleware.cors import CORSMiddleware
from fastapi.security import OAuth2PasswordBearer
from grpclib.server import Server
from jose import JWTError, jwt
from pydantic import BaseModel
from pydantic_settings import BaseSettings
import grpc
import json

from cowcow_pb2 import (
    Chunk,
    UploadResponse,
    UploadRequest,
    UploadStatus,
    BalanceRequest,
    BalanceResponse,
    Transaction,
    HistoryRequest,
)
from cowcow_grpc import UploadServiceBase, RewardServiceBase
import auth
import database
from models import User, Recording, Token, UploadQueue
from database import get_db
from sqlalchemy.orm import Session

# Configuration
class Settings(BaseSettings):
    jwt_secret: str = "test-secret-key-for-development-only"
    jwt_algorithm: str = "HS256"
    jwt_expire_minutes: int = 1440  # 24 hours
    r2_access_key: str = "test-access-key"
    r2_secret_key: str = "test-secret-key"
    r2_endpoint: str = "https://test-endpoint.com"
    r2_bucket: str = "test-bucket"
    database_url: str = "sqlite:///./test_db.sqlite"

    class Config:
        env_file = ".env"

settings = Settings()

# FastAPI app
app = FastAPI(title="Cowcow CLI API", description="Offline-first speech data collection API")
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="token")

# CORS middleware configuration
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # TODO: Restrict in production
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Include routers
app.include_router(auth.router, prefix="/auth", tags=["authentication"])

# Initialize database
database.init_db()

# Upload configuration
UPLOAD_DIR = os.getenv("UPLOAD_DIR", "uploads")
os.makedirs(UPLOAD_DIR, exist_ok=True)

# Token reward configuration
TOKENS_PER_MINUTE = 10
MIN_RECORDING_LENGTH = 1  # seconds

# S3 client for R2
s3 = boto3.client(
    "s3",
    endpoint_url=settings.r2_endpoint,
    aws_access_key_id=settings.r2_access_key,
    aws_secret_access_key=settings.r2_secret_key,
)

# JWT functions
def create_access_token(data: dict, expires_delta: Optional[timedelta] = None):
    to_encode = data.copy()
    if expires_delta:
        expire = datetime.utcnow() + expires_delta
    else:
        expire = datetime.utcnow() + timedelta(minutes=15)
    to_encode.update({"exp": expire})
    encoded_jwt = jwt.encode(
        to_encode, settings.jwt_secret, algorithm=settings.jwt_algorithm
    )
    return encoded_jwt

async def get_current_user(token: str = Depends(oauth2_scheme)):
    credentials_exception = HTTPException(
        status_code=401,
        detail="Could not validate credentials",
        headers={"WWW-Authenticate": "Bearer"},
    )
    try:
        payload = jwt.decode(
            token, settings.jwt_secret, algorithms=[settings.jwt_algorithm]
        )
        username: str = payload.get("sub")
        if username is None:
            raise credentials_exception
    except JWTError:
        raise credentials_exception
    
    # Get user from database
    db = next(get_db())
    user = db.query(User).filter(User.username == username).first()
    if user is None:
        raise credentials_exception
    return user

async def get_current_user_by_api_key_or_token(
    api_key: Optional[str] = None,
    token: Optional[str] = None,
    db: Session = Depends(get_db)
):
    """Get current user by API key or Bearer token"""
    credentials_exception = HTTPException(
        status_code=401,
        detail="Could not validate credentials",
        headers={"WWW-Authenticate": "Bearer"},
    )
    
    # Try API key first
    if api_key:
        user = db.query(User).filter(User.api_key == api_key).first()
        if user:
            return user
    
    # Try Bearer token
    if token:
        try:
            payload = jwt.decode(
                token, settings.jwt_secret, algorithms=[settings.jwt_algorithm]
            )
            username: str = payload.get("sub")
            if username:
                user = db.query(User).filter(User.username == username).first()
                if user:
                    return user
        except JWTError:
            pass
    
    raise credentials_exception

async def get_current_user_multi_auth(
    request: Request,
    db: Session = Depends(get_db)
):
    """Get current user by API key or Bearer token from request"""
    # Try to get API key from X-API-Key header
    api_key = request.headers.get("X-API-Key")
    
    # Try to get Bearer token from Authorization header
    auth_header = request.headers.get("Authorization")
    token = None
    if auth_header and auth_header.startswith("Bearer "):
        token = auth_header.split(" ")[1]
    
    return await get_current_user_by_api_key_or_token(api_key, token, db)

# gRPC service implementation
class UploadServiceImpl(UploadServiceBase):
    async def UploadChunk(self, stream):
        """Handle chunked upload of recordings."""
        chunks = []
        recording_id = None
        lang = None
        user_id = None
        
        async for chunk in stream:
            if not recording_id:
                recording_id = chunk.recording_id
                lang = chunk.lang
                # TODO: Get user_id from authentication context
            
            # Store chunk in R2
            key = f"{lang}/{user_id}/{recording_id}/{chunk.sequence}"
            s3.put_object(
                Bucket=settings.r2_bucket,
                Key=key,
                Body=chunk.data,
            )
            chunks.append(chunk)
        
        # TODO: Update database with recording metadata
        # TODO: Award tokens based on QC metrics
        
        return UploadResponse(
            success=True,
            tokens_awarded=3,  # Example token award
        )
    
    async def GetUploadStatus(self, request: UploadRequest) -> UploadStatus:
        """Get the status of an upload."""
        # TODO: Implement status lookup from database
        return UploadStatus(
            status=UploadStatus.Status.COMPLETED,
            progress=100,
            chunks_uploaded=1,
            total_chunks=1,
        )

class RewardServiceImpl(RewardServiceBase):
    async def GetBalance(self, request: BalanceRequest) -> BalanceResponse:
        """Get the token balance for a contributor."""
        # TODO: Implement balance lookup from database
        return BalanceResponse(
            balance=0,
            total_earned=0,
            total_spent=0,
        )
    
    async def GetHistory(self, request: HistoryRequest):
        """Get the transaction history for a contributor."""
        # TODO: Implement history lookup from database
        yield Transaction(
            transaction_id="example",
            type=Transaction.Type.EARNED,
            amount=3,
            timestamp=int(datetime.utcnow().timestamp()),
            description="Recording upload",
        )

# REST API endpoints
@app.post("/recordings/upload")
async def upload_recording(
    recording_id: str = Form(...),
    lang: str = Form(...),
    qc_metrics: str = Form(...),
    file_path: str = Form(...),
    file: UploadFile = File(...),
    current_user: User = Depends(get_current_user_multi_auth),
    db: Session = Depends(get_db)
):
    """Upload a recording and process QC metrics."""
    try:
        # Save the uploaded file
        upload_path = os.path.join(UPLOAD_DIR, f"{recording_id}.wav")
        with open(upload_path, "wb") as buffer:
            content = await file.read()
            buffer.write(content)
        
        # Check if recording already exists
        existing_recording = db.query(Recording).filter(Recording.id == recording_id).first()
        if existing_recording:
            # Update existing recording with uploaded file
            existing_recording.file_path = upload_path
            existing_recording.status = "processing"
            recording = existing_recording
        else:
            # Create new recording record
            recording = Recording(
                id=recording_id,
                user_id=current_user.id,
                lang=lang,
                qc_metrics=qc_metrics,
                file_path=upload_path,
                status="processing"
            )
            db.add(recording)
        
        db.commit()
        db.refresh(recording)

        # Process QC metrics
        metrics = json.loads(qc_metrics)
        
        # Calculate duration from uploaded file
        import wave
        try:
            with wave.open(upload_path, 'rb') as wav_file:
                frames = wav_file.getnframes()
                sample_rate = wav_file.getframerate()
                duration = frames / sample_rate
        except Exception as e:
            # Fallback: estimate duration from file size (approximate)
            file_size = os.path.getsize(upload_path)
            # Rough estimate: 16-bit mono at 48kHz ≈ 96kB per second
            duration = (file_size - 44) / (48000 * 2)  # Subtract WAV header, assume 16-bit mono
        
        if duration >= MIN_RECORDING_LENGTH:
            # Calculate tokens based on duration
            tokens = int((duration / 60) * TOKENS_PER_MINUTE)
            
            # Add tokens to user's balance
            token = Token(
                id=recording_id,
                user_id=current_user.id,
                amount=tokens,
                type="recording",
                description=f"Recording reward for {duration:.1f}s clip",
                recording_id=recording_id
            )
            db.add(token)
            
            # Update recording status
            recording.status = "completed"
            recording.uploaded_at = datetime.utcnow()
            
            db.commit()
            
            return {
                "status": "success",
                "tokens_awarded": tokens,
                "recording_id": recording_id
            }
        else:
            recording.status = "failed"
            db.commit()
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Recording too short"
            )
    except Exception as e:
        db.rollback()
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=str(e)
        )

@app.get("/recordings")
async def list_recordings(
    current_user: User = Depends(get_current_user_multi_auth),
    db: Session = Depends(get_db)
):
    """List user's recordings."""
    recordings = db.query(Recording).filter(
        Recording.user_id == current_user.id
    ).order_by(Recording.created_at.desc()).all()
    
    return recordings

@app.get("/tokens/balance")
async def get_token_balance(
    current_user: User = Depends(get_current_user_multi_auth),
    db: Session = Depends(get_db)
):
    """Get user's token balance."""
    tokens = db.query(Token).filter(
        Token.user_id == current_user.id
    ).all()
    
    balance = sum(token.amount for token in tokens)
    return {"balance": balance}

@app.get("/tokens/history")
async def get_token_history(
    current_user: User = Depends(get_current_user_multi_auth),
    db: Session = Depends(get_db)
):
    """Get user's token transaction history."""
    tokens = db.query(Token).filter(
        Token.user_id == current_user.id
    ).order_by(Token.created_at.desc()).all()
    
    return tokens

@app.get("/health")
async def health_check():
    """Health check endpoint."""
    return {"status": "healthy", "timestamp": datetime.utcnow().isoformat()}

# Server startup
async def serve():
    server = Server([UploadServiceImpl(), RewardServiceImpl()])
    await server.start("0.0.0.0", 50051)
    await server.wait_closed()

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000) 