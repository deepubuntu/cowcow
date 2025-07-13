# Combined gRPC module for Cowcow
# This module re-exports all gRPC service classes from upload and reward modules

from upload_pb2_grpc import *
from reward_pb2_grpc import *

# Create aliases for the expected base class names
UploadServiceBase = UploadServiceServicer
RewardServiceBase = RewardServiceServicer 