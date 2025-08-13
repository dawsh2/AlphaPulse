"""
Workspace Service - Business logic for workspace file operations
Handles secure file operations within sandboxed workspace directory
"""
import os
import shutil
import logging
from pathlib import Path
from typing import Dict, Any, List, Optional
from datetime import datetime

logger = logging.getLogger(__name__)

class WorkspaceService:
    """Service layer for workspace operations"""
    
    # Security: Allowed file extensions for editing
    ALLOWED_EXTENSIONS = {
        '.py', '.json', '.yaml', '.yml', '.txt', '.md', '.csv', 
        '.ipynb', '.html', '.css', '.js', '.ts', '.jsx', '.tsx',
        '.sh', '.env', '.gitignore', '.dockerfile', '.toml', '.ini'
    }
    
    # Security: Restricted directories (cannot access)
    RESTRICTED_PATHS = {'__pycache__', '.git', 'node_modules', '.env.local', '.venv', 'venv'}
    
    def __init__(self, workspace_root: Optional[Path] = None):
        """Initialize workspace service
        
        Args:
            workspace_root: Root directory for workspace. Defaults to backend/workspace
        """
        if workspace_root is None:
            self.workspace_root = Path(__file__).parent.parent / "workspace"
        else:
            self.workspace_root = Path(workspace_root)
        
        # Ensure workspace exists
        self.workspace_root.mkdir(exist_ok=True, parents=True)
        logger.info(f"Workspace initialized at: {self.workspace_root}")
    
    def is_safe_path(self, path: Path) -> bool:
        """Check if a path is safe to access
        
        Args:
            path: Path to validate
            
        Returns:
            True if path is safe, False otherwise
        """
        try:
            # Resolve to absolute path
            resolved = path.resolve()
            workspace_resolved = self.workspace_root.resolve()
            
            # Check if path is within workspace
            if not str(resolved).startswith(str(workspace_resolved)):
                logger.warning(f"Path outside workspace attempted: {resolved}")
                return False
            
            # Check for restricted directories
            for part in resolved.parts:
                if part in self.RESTRICTED_PATHS:
                    logger.warning(f"Restricted path accessed: {part}")
                    return False
            
            return True
        except Exception as e:
            logger.error(f"Path validation error: {e}")
            return False
    
    def get_file_info(self, file_path: Path) -> Dict[str, Any]:
        """Get file metadata
        
        Args:
            file_path: Path to file
            
        Returns:
            Dictionary with file metadata
        """
        try:
            stat = file_path.stat()
            rel_path = file_path.relative_to(self.workspace_root)
            
            return {
                'name': file_path.name,
                'path': str(rel_path).replace('\\', '/'),  # Normalize path separators
                'type': 'folder' if file_path.is_dir() else 'file',
                'size': stat.st_size if file_path.is_file() else None,
                'modified': datetime.fromtimestamp(stat.st_mtime).isoformat(),
                'extension': file_path.suffix if file_path.is_file() else None,
                'permissions': oct(stat.st_mode)[-3:] if hasattr(stat, 'st_mode') else None
            }
        except Exception as e:
            logger.error(f"Error getting file info for {file_path}: {e}")
            raise
    
    async def list_files(self, path: str = "") -> List[Dict[str, Any]]:
        """List files in directory
        
        Args:
            path: Relative path within workspace
            
        Returns:
            List of file metadata dictionaries
            
        Raises:
            ValueError: If path is invalid or unsafe
            FileNotFoundError: If path doesn't exist
        """
        target_path = self.workspace_root / path
        
        if not self.is_safe_path(target_path):
            raise ValueError(f"Invalid or unsafe path: {path}")
        
        if not target_path.exists():
            raise FileNotFoundError(f"Path not found: {path}")
        
        if not target_path.is_dir():
            raise ValueError(f"Path is not a directory: {path}")
        
        files = []
        for item in sorted(target_path.iterdir()):
            # Skip hidden files (except specific allowed ones)
            if item.name.startswith('.') and item.name not in ['.env', '.gitignore']:
                continue
            
            # Skip restricted paths
            if item.name in self.RESTRICTED_PATHS:
                continue
            
            try:
                file_info = self.get_file_info(item)
                files.append(file_info)
            except Exception as e:
                logger.warning(f"Skipping file {item}: {e}")
                continue
        
        return files
    
    async def read_file(self, filepath: str) -> Dict[str, Any]:
        """Read file contents
        
        Args:
            filepath: Relative path to file
            
        Returns:
            Dictionary with file content and metadata
            
        Raises:
            ValueError: If path is invalid or file type not allowed
            FileNotFoundError: If file doesn't exist
        """
        target_path = self.workspace_root / filepath
        
        if not self.is_safe_path(target_path):
            raise ValueError(f"Invalid or unsafe path: {filepath}")
        
        if not target_path.exists():
            raise FileNotFoundError(f"File not found: {filepath}")
        
        if target_path.is_dir():
            raise ValueError(f"Path is a directory, not a file: {filepath}")
        
        # Check file extension
        if target_path.suffix not in self.ALLOWED_EXTENSIONS:
            raise ValueError(f"File type not allowed for editing: {target_path.suffix}")
        
        try:
            # Determine if file is binary
            is_binary = target_path.suffix in {'.ipynb'}
            
            if is_binary:
                with open(target_path, 'rb') as f:
                    content = f.read()
                # For notebooks, parse as JSON
                if target_path.suffix == '.ipynb':
                    import json
                    content = json.loads(content)
            else:
                with open(target_path, 'r', encoding='utf-8') as f:
                    content = f.read()
            
            return {
                'content': content,
                'path': filepath,
                'name': target_path.name,
                'extension': target_path.suffix,
                'size': target_path.stat().st_size,
                'modified': datetime.fromtimestamp(target_path.stat().st_mtime).isoformat(),
                'is_binary': is_binary
            }
        except Exception as e:
            logger.error(f"Error reading file {filepath}: {e}")
            raise
    
    async def write_file(self, filepath: str, content: Any) -> Dict[str, Any]:
        """Write or create file
        
        Args:
            filepath: Relative path to file
            content: File content (string or dict for JSON files)
            
        Returns:
            Dictionary with file metadata
            
        Raises:
            ValueError: If path is invalid or file type not allowed
        """
        target_path = self.workspace_root / filepath
        
        # For new files, check parent directory
        if not target_path.exists():
            parent_path = target_path.parent
            if not self.is_safe_path(parent_path):
                raise ValueError(f"Invalid or unsafe parent path: {parent_path}")
            
            # Create parent directories if needed
            parent_path.mkdir(parents=True, exist_ok=True)
        else:
            if not self.is_safe_path(target_path):
                raise ValueError(f"Invalid or unsafe path: {filepath}")
        
        # Check file extension
        if target_path.suffix not in self.ALLOWED_EXTENSIONS:
            raise ValueError(f"File type not allowed: {target_path.suffix}")
        
        try:
            # Handle different content types
            if target_path.suffix == '.ipynb' and isinstance(content, dict):
                import json
                with open(target_path, 'w', encoding='utf-8') as f:
                    json.dump(content, f, indent=2)
            elif isinstance(content, str):
                with open(target_path, 'w', encoding='utf-8') as f:
                    f.write(content)
            else:
                raise ValueError(f"Invalid content type for file: {type(content)}")
            
            logger.info(f"File written: {filepath}")
            return self.get_file_info(target_path)
            
        except Exception as e:
            logger.error(f"Error writing file {filepath}: {e}")
            raise
    
    async def delete_file(self, filepath: str) -> Dict[str, str]:
        """Delete file or directory
        
        Args:
            filepath: Relative path to file or directory
            
        Returns:
            Success message
            
        Raises:
            ValueError: If path is invalid
            FileNotFoundError: If file doesn't exist
        """
        target_path = self.workspace_root / filepath
        
        if not self.is_safe_path(target_path):
            raise ValueError(f"Invalid or unsafe path: {filepath}")
        
        if not target_path.exists():
            raise FileNotFoundError(f"File not found: {filepath}")
        
        try:
            if target_path.is_dir():
                shutil.rmtree(target_path)
                logger.info(f"Directory deleted: {filepath}")
                return {"message": f"Directory deleted: {filepath}"}
            else:
                target_path.unlink()
                logger.info(f"File deleted: {filepath}")
                return {"message": f"File deleted: {filepath}"}
                
        except Exception as e:
            logger.error(f"Error deleting {filepath}: {e}")
            raise
    
    async def rename_file(self, old_path: str, new_path: str) -> Dict[str, Any]:
        """Rename or move file
        
        Args:
            old_path: Current relative path
            new_path: New relative path
            
        Returns:
            Dictionary with new file metadata
            
        Raises:
            ValueError: If paths are invalid
            FileNotFoundError: If source doesn't exist
            FileExistsError: If destination already exists
        """
        old_target = self.workspace_root / old_path
        new_target = self.workspace_root / new_path
        
        if not self.is_safe_path(old_target):
            raise ValueError(f"Invalid or unsafe source path: {old_path}")
        
        if not self.is_safe_path(new_target):
            raise ValueError(f"Invalid or unsafe destination path: {new_path}")
        
        if not old_target.exists():
            raise FileNotFoundError(f"Source file not found: {old_path}")
        
        if new_target.exists():
            raise FileExistsError(f"Destination already exists: {new_path}")
        
        try:
            # Create parent directories for destination if needed
            new_target.parent.mkdir(parents=True, exist_ok=True)
            
            # Rename/move the file
            old_target.rename(new_target)
            logger.info(f"File renamed: {old_path} -> {new_path}")
            
            return self.get_file_info(new_target)
            
        except Exception as e:
            logger.error(f"Error renaming {old_path} to {new_path}: {e}")
            raise
    
    async def create_directory(self, path: str) -> Dict[str, Any]:
        """Create a new directory
        
        Args:
            path: Relative path for new directory
            
        Returns:
            Dictionary with directory metadata
            
        Raises:
            ValueError: If path is invalid
            FileExistsError: If directory already exists
        """
        target_path = self.workspace_root / path
        
        if not self.is_safe_path(target_path):
            raise ValueError(f"Invalid or unsafe path: {path}")
        
        if target_path.exists():
            raise FileExistsError(f"Path already exists: {path}")
        
        try:
            target_path.mkdir(parents=True, exist_ok=False)
            logger.info(f"Directory created: {path}")
            return self.get_file_info(target_path)
            
        except Exception as e:
            logger.error(f"Error creating directory {path}: {e}")
            raise