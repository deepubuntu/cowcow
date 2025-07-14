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
import uuid

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
        db = next(get_db())
        try:
            # Get user from contributor_id (assuming it's user_id)
            user = db.query(User).filter(User.id == int(request.contributor_id)).first()
            if not user:
                raise Exception(f"User not found: {request.contributor_id}")
            
            # Calculate balance from tokens table
            tokens = db.query(Token).filter(Token.user_id == user.id).all()
            
            total_earned = sum(token.amount for token in tokens if token.amount > 0)
            total_spent = sum(abs(token.amount) for token in tokens if token.amount < 0)
            balance = total_earned - total_spent
            
            return BalanceResponse(
                balance=balance,
                total_earned=total_earned,
                total_spent=total_spent,
            )
        finally:
            db.close()
    
    async def GetHistory(self, request: HistoryRequest):
        """Get the transaction history for a contributor."""
        db = next(get_db())
        try:
            # Get user from contributor_id (assuming it's user_id)
            user = db.query(User).filter(User.id == int(request.contributor_id)).first()
            if not user:
                raise Exception(f"User not found: {request.contributor_id}")
            
            # Build query with optional time filtering
            query = db.query(Token).filter(Token.user_id == user.id)
            
            if request.start_time:
                start_dt = datetime.fromtimestamp(request.start_time)
                query = query.filter(Token.created_at >= start_dt)
            
            if request.end_time:
                end_dt = datetime.fromtimestamp(request.end_time)
                query = query.filter(Token.created_at <= end_dt)
            
            # Order by most recent first
            tokens = query.order_by(Token.created_at.desc()).all()
            
            # Calculate running balance for each transaction
            running_balance = 0
            for token in reversed(tokens):  # Calculate from oldest to newest
                running_balance += token.amount
            
            # Yield transactions in reverse order (newest first)
            for token in tokens:
                yield Transaction(
                    transaction_id=token.id,
                    type=Transaction.Type.EARNED if token.amount > 0 else Transaction.Type.SPENT,
                    amount=token.amount,
                    timestamp=int(token.created_at.timestamp()),
                    description=token.description or f"{token.type} transaction",
                )
        finally:
            db.close()

# REST API endpoints
@app.post("/recordings/upload")
async def upload_recording(
    recording_id: str = Form(...),
    lang: str = Form(...),
    qc_metrics: str = Form(...),
    file_path: str = Form(...),
    current_user: User = Depends(get_current_user_multi_auth),
    db: Session = Depends(get_db)
):
    """Upload a recording and award tokens based on quality."""
    try:
        # Parse QC metrics
        metrics = json.loads(qc_metrics)
        
        # Save recording to database
        recording = Recording(
            id=recording_id,
            user_id=current_user.id,
            lang=lang,
            qc_metrics=qc_metrics,
            file_path=file_path,
            status="completed"
        )
        db.add(recording)
        
        # Calculate token reward based on QC metrics
        base_tokens = TOKENS_PER_MINUTE  # Base reward
        
        # Bonus for high quality
        snr_db = metrics.get("snr_db", 0)
        clipping_pct = metrics.get("clipping_pct", 100)
        vad_ratio = metrics.get("vad_ratio", 0)
        
        bonus_tokens = 0
        if snr_db > 20:  # High SNR bonus
            bonus_tokens += 2
        if clipping_pct < 1:  # Low clipping bonus
            bonus_tokens += 1
        if vad_ratio > 0.3:  # Good voice activity bonus
            bonus_tokens += 1
        
        total_tokens = base_tokens + bonus_tokens
        
        # Award tokens
        token_record = Token(
            id=str(uuid.uuid4()),
            user_id=current_user.id,
            amount=total_tokens,
            type="recording",
            description=f"Recording upload: {lang} (SNR: {snr_db:.1f}dB, Clipping: {clipping_pct:.1f}%)",
            recording_id=recording_id
        )
        db.add(token_record)
        
        db.commit()
        
        return {
            "status": "success",
            "recording_id": recording_id,
            "tokens_awarded": total_tokens,
            "message": f"Recording uploaded successfully! Earned {total_tokens} tokens."
        }
        
    except Exception as e:
        db.rollback()
        raise HTTPException(status_code=400, detail=str(e))

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
    """Get the current token balance for the authenticated user."""
    # Calculate balance from tokens table
    tokens = db.query(Token).filter(Token.user_id == current_user.id).all()
    
    total_earned = sum(token.amount for token in tokens if token.amount > 0)
    total_spent = sum(abs(token.amount) for token in tokens if token.amount < 0)
    balance = total_earned - total_spent
    
    return {
        "balance": balance,
        "total_earned": total_earned,
        "total_spent": total_spent
    }

@app.get("/tokens/history")
async def get_token_history(
    days: int = 30,
    current_user: User = Depends(get_current_user_multi_auth),
    db: Session = Depends(get_db)
):
    """Get the token transaction history for the authenticated user."""
    # Calculate start date
    start_date = datetime.utcnow() - timedelta(days=days)
    
    # Query tokens with date filtering
    tokens = db.query(Token).filter(
        Token.user_id == current_user.id,
        Token.created_at >= start_date
    ).order_by(Token.created_at.desc()).all()
    
    # Convert to API response format
    transactions = []
    for token in tokens:
        transactions.append({
            "id": token.id,
            "transaction_type": token.type,
            "amount": token.amount,
            "balance": 0,  # Will be calculated below
            "date": token.created_at.isoformat(),
            "notes": token.description or f"{token.type} transaction"
        })
    
    # Calculate running balance for each transaction
    # Start with current balance and work backwards
    all_tokens = db.query(Token).filter(Token.user_id == current_user.id).all()
    current_balance = sum(token.amount for token in all_tokens)
    
    for transaction in transactions:
        transaction["balance"] = current_balance
        current_balance -= transaction["amount"]
    
    return transactions

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