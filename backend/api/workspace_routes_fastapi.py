"""
Workspace API Routes for FastAPI - File system operations for development environment
"""
from fastapi import APIRouter, HTTPException, Depends, Query, Path as PathParam
from pydantic import BaseModel, Field
from typing import Optional, List, Dict, Any, Union
import logging
from pathlib import Path

from services.workspace_service import WorkspaceService

# Setup logging
logger = logging.getLogger(__name__)

# Create router
router = APIRouter(
    prefix="/api/workspace",
    tags=["workspace"],
    responses={404: {"description": "Not found"}},
)

# Pydantic models
class FileInfo(BaseModel):
    """File or directory information"""
    name: str
    path: str
    type: str = Field(..., pattern="^(file|folder)$")
    size: Optional[int] = None
    modified: str
    extension: Optional[str] = None
    permissions: Optional[str] = None

class FileListResponse(BaseModel):
    """Response for file listing"""
    files: List[FileInfo]
    path: str
    total: int

class FileContent(BaseModel):
    """File content for reading/writing"""
    content: Union[str, Dict[str, Any]]
    path: str
    name: str
    extension: str
    size: int
    modified: str
    is_binary: bool = False

class WriteFileRequest(BaseModel):
    """Request to write file content"""
    content: Union[str, Dict[str, Any]]

class RenameRequest(BaseModel):
    """Request to rename/move file"""
    old_path: str
    new_path: str

class CreateDirectoryRequest(BaseModel):
    """Request to create directory"""
    path: str

class OperationResponse(BaseModel):
    """Generic operation response"""
    message: str
    path: Optional[str] = None

# Dependency to get workspace service
def get_workspace_service() -> WorkspaceService:
    """Get workspace service instance"""
    return WorkspaceService()

# API Endpoints
@router.get("/files", response_model=FileListResponse)
async def list_files(
    path: str = Query("", description="Relative path within workspace"),
    service: WorkspaceService = Depends(get_workspace_service)
):
    """
    List files and directories in the workspace
    
    Args:
        path: Relative path within workspace (empty string for root)
        
    Returns:
        List of files and directories with metadata
        
    Raises:
        HTTPException: If path is invalid or not found
    """
    try:
        files = await service.list_files(path)
        return FileListResponse(
            files=files,
            path=path,
            total=len(files)
        )
    except ValueError as e:
        raise HTTPException(status_code=403, detail=str(e))
    except FileNotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    except Exception as e:
        logger.error(f"Error listing files: {e}")
        raise HTTPException(status_code=500, detail="Internal server error")

@router.get("/file/{filepath:path}", response_model=FileContent)
async def read_file(
    filepath: str = PathParam(..., description="File path relative to workspace"),
    service: WorkspaceService = Depends(get_workspace_service)
):
    """
    Read file contents
    
    Args:
        filepath: Path to file relative to workspace root
        
    Returns:
        File content and metadata
        
    Raises:
        HTTPException: If file not found or not allowed
    """
    try:
        result = await service.read_file(filepath)
        return FileContent(**result)
    except ValueError as e:
        raise HTTPException(status_code=403, detail=str(e))
    except FileNotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    except Exception as e:
        logger.error(f"Error reading file {filepath}: {e}")
        raise HTTPException(status_code=500, detail="Internal server error")

@router.put("/file/{filepath:path}", response_model=FileInfo)
async def write_file(
    filepath: str = PathParam(..., description="File path relative to workspace"),
    request: WriteFileRequest = ...,
    service: WorkspaceService = Depends(get_workspace_service)
):
    """
    Write or create a file
    
    Args:
        filepath: Path to file relative to workspace root
        request: File content to write
        
    Returns:
        Updated file metadata
        
    Raises:
        HTTPException: If path is invalid or write fails
    """
    try:
        result = await service.write_file(filepath, request.content)
        return FileInfo(**result)
    except ValueError as e:
        raise HTTPException(status_code=403, detail=str(e))
    except Exception as e:
        logger.error(f"Error writing file {filepath}: {e}")
        raise HTTPException(status_code=500, detail="Internal server error")

@router.post("/file/{filepath:path}", response_model=FileInfo)
async def create_file(
    filepath: str = PathParam(..., description="File path relative to workspace"),
    request: WriteFileRequest = ...,
    service: WorkspaceService = Depends(get_workspace_service)
):
    """
    Create a new file (alias for write_file for new files)
    
    Args:
        filepath: Path to new file relative to workspace root
        request: File content
        
    Returns:
        New file metadata
        
    Raises:
        HTTPException: If file already exists or creation fails
    """
    try:
        # Check if file already exists
        target_path = Path(service.workspace_root) / filepath
        if target_path.exists():
            raise HTTPException(status_code=409, detail="File already exists")
        
        result = await service.write_file(filepath, request.content)
        return FileInfo(**result)
    except HTTPException:
        raise
    except ValueError as e:
        raise HTTPException(status_code=403, detail=str(e))
    except Exception as e:
        logger.error(f"Error creating file {filepath}: {e}")
        raise HTTPException(status_code=500, detail="Internal server error")

@router.delete("/file/{filepath:path}", response_model=OperationResponse)
async def delete_file(
    filepath: str = PathParam(..., description="File or directory path relative to workspace"),
    service: WorkspaceService = Depends(get_workspace_service)
):
    """
    Delete a file or directory
    
    Args:
        filepath: Path to file or directory relative to workspace root
        
    Returns:
        Success message
        
    Raises:
        HTTPException: If file not found or deletion fails
    """
    try:
        result = await service.delete_file(filepath)
        return OperationResponse(**result, path=filepath)
    except ValueError as e:
        raise HTTPException(status_code=403, detail=str(e))
    except FileNotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    except Exception as e:
        logger.error(f"Error deleting {filepath}: {e}")
        raise HTTPException(status_code=500, detail="Internal server error")

@router.post("/rename", response_model=FileInfo)
async def rename_file(
    request: RenameRequest,
    service: WorkspaceService = Depends(get_workspace_service)
):
    """
    Rename or move a file/directory
    
    Args:
        request: Old and new paths
        
    Returns:
        Updated file metadata with new path
        
    Raises:
        HTTPException: If source not found or destination exists
    """
    try:
        result = await service.rename_file(request.old_path, request.new_path)
        return FileInfo(**result)
    except ValueError as e:
        raise HTTPException(status_code=403, detail=str(e))
    except FileNotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    except FileExistsError as e:
        raise HTTPException(status_code=409, detail=str(e))
    except Exception as e:
        logger.error(f"Error renaming {request.old_path} to {request.new_path}: {e}")
        raise HTTPException(status_code=500, detail="Internal server error")

@router.post("/directory", response_model=FileInfo)
async def create_directory(
    request: CreateDirectoryRequest,
    service: WorkspaceService = Depends(get_workspace_service)
):
    """
    Create a new directory
    
    Args:
        request: Directory path to create
        
    Returns:
        New directory metadata
        
    Raises:
        HTTPException: If directory already exists or creation fails
    """
    try:
        result = await service.create_directory(request.path)
        return FileInfo(**result)
    except ValueError as e:
        raise HTTPException(status_code=403, detail=str(e))
    except FileExistsError as e:
        raise HTTPException(status_code=409, detail=str(e))
    except Exception as e:
        logger.error(f"Error creating directory {request.path}: {e}")
        raise HTTPException(status_code=500, detail="Internal server error")

# Additional helper endpoints
@router.get("/health")
async def workspace_health(
    service: WorkspaceService = Depends(get_workspace_service)
):
    """Health check for workspace service"""
    workspace_path = service.workspace_root
    
    return {
        "service": "workspace",
        "status": "healthy",
        "workspace_root": str(workspace_path),
        "workspace_exists": workspace_path.exists(),
        "workspace_writable": os.access(workspace_path, os.W_OK) if workspace_path.exists() else False
    }

@router.get("/stats")
async def workspace_stats(
    service: WorkspaceService = Depends(get_workspace_service)
):
    """Get workspace statistics"""
    try:
        # Count files and directories
        total_files = 0
        total_dirs = 0
        total_size = 0
        
        for item in service.workspace_root.rglob("*"):
            if item.is_file():
                total_files += 1
                total_size += item.stat().st_size
            elif item.is_dir():
                total_dirs += 1
        
        return {
            "total_files": total_files,
            "total_directories": total_dirs,
            "total_size_bytes": total_size,
            "total_size_mb": round(total_size / (1024 * 1024), 2),
            "workspace_path": str(service.workspace_root)
        }
    except Exception as e:
        logger.error(f"Error getting workspace stats: {e}")
        raise HTTPException(status_code=500, detail="Internal server error")

# Import os for health check
import os