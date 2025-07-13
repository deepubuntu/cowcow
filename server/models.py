from datetime import datetime
from typing import Optional
from sqlalchemy import Column, Integer, String, DateTime, Boolean, ForeignKey, Text
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import relationship
import bcrypt

Base = declarative_base()

class User(Base):
    __tablename__ = 'users'

    id = Column(Integer, primary_key=True)
    username = Column(String(50), unique=True, nullable=False)
    email = Column(String(120), unique=True, nullable=False)
    password_hash = Column(String(60), nullable=False)
    is_active = Column(Boolean, default=True)
    created_at = Column(DateTime, default=datetime.utcnow)
    last_login = Column(DateTime)
    api_key = Column(String(64), unique=True)
    role = Column(String(20), default='user')  # user, admin, moderator

    recordings = relationship("Recording", back_populates="user")
    tokens = relationship("Token", back_populates="user")

    def set_password(self, password: str) -> None:
        """Hash and set the user's password."""
        salt = bcrypt.gensalt()
        self.password_hash = bcrypt.hashpw(password.encode(), salt).decode()

    def check_password(self, password: str) -> bool:
        """Check if the provided password matches the hash."""
        return bcrypt.checkpw(password.encode(), self.password_hash.encode())

class Recording(Base):
    __tablename__ = 'recordings'

    id = Column(String(36), primary_key=True)
    user_id = Column(Integer, ForeignKey('users.id'), nullable=False)
    lang = Column(String(10), nullable=False)
    prompt = Column(Text)
    qc_metrics = Column(Text, nullable=False)
    file_path = Column(String(255), nullable=False)
    created_at = Column(DateTime, default=datetime.utcnow)
    uploaded_at = Column(DateTime)
    status = Column(String(20), default='pending')  # pending, processing, completed, failed

    user = relationship("User", back_populates="recordings")

class Token(Base):
    __tablename__ = 'tokens'

    id = Column(String(36), primary_key=True)
    user_id = Column(Integer, ForeignKey('users.id'), nullable=False)
    amount = Column(Integer, nullable=False)
    type = Column(String(20), nullable=False)  # recording, bonus, referral
    description = Column(Text)
    recording_id = Column(String(36), ForeignKey('recordings.id'))
    created_at = Column(DateTime, default=datetime.utcnow)

    user = relationship("User", back_populates="tokens")
    recording = relationship("Recording")

class UploadQueue(Base):
    __tablename__ = 'upload_queue'

    recording_id = Column(String(36), ForeignKey('recordings.id'), primary_key=True)
    attempts = Column(Integer, default=0)
    last_attempt = Column(DateTime)
    error_message = Column(Text)

    recording = relationship("Recording") 