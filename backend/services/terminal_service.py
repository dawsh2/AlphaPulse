"""
Terminal Service Layer - Business logic for terminal sessions
Handles PTY creation, session management, and terminal I/O
"""
import os
import pty
import select
import subprocess
import threading
import time
import uuid
from typing import Dict, Optional, List, Any
import logging

logger = logging.getLogger(__name__)

class TerminalSession:
    """Individual terminal session with PTY support"""
    
    def __init__(self, session_id: str, shell: str = None):
        self.session_id = session_id
        self.shell = shell or os.environ.get('SHELL', '/bin/bash')
        self.master_fd = None
        self.slave_fd = None
        self.process = None
        self.running = False
        self.cwd = os.path.expanduser('~')
        self.created_at = time.time()
        self.last_activity = time.time()
        
    def start(self) -> bool:
        """Start a new PTY session"""
        try:
            # Create PTY
            self.master_fd, self.slave_fd = pty.openpty()
            
            # Start shell process
            env = os.environ.copy()
            env['TERM'] = 'xterm-256color'
            env['PS1'] = '\\[\\033[96m\\]\\w\\[\\033[0m\\]$ '  # Phosphor blue prompt
            
            self.process = subprocess.Popen(
                [self.shell],
                stdin=self.slave_fd,
                stdout=self.slave_fd,
                stderr=self.slave_fd,
                env=env,
                cwd=self.cwd,
                preexec_fn=os.setsid
            )
            
            # Close slave fd in parent process
            os.close(self.slave_fd)
            self.running = True
            
            logger.info(f"Terminal session {self.session_id} started with PID {self.process.pid}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to start terminal session {self.session_id}: {e}")
            return False
    
    def write(self, data: str) -> bool:
        """Write data to the terminal"""
        if self.master_fd and self.running:
            try:
                os.write(self.master_fd, data.encode('utf-8'))
                self.last_activity = time.time()
                return True
            except OSError as e:
                logger.error(f"Error writing to terminal {self.session_id}: {e}")
                self.running = False
                return False
        return False
    
    def read(self, timeout: float = 0.1) -> Optional[str]:
        """Read available data from the terminal"""
        if not self.master_fd or not self.running:
            return None
        
        try:
            # Use select to check if data is available
            ready, _, _ = select.select([self.master_fd], [], [], timeout)
            if ready:
                data = os.read(self.master_fd, 1024)
                self.last_activity = time.time()
                return data.decode('utf-8', errors='ignore')
        except OSError as e:
            logger.error(f"Error reading from terminal {self.session_id}: {e}")
            self.running = False
        
        return None
    
    def resize(self, cols: int, rows: int) -> bool:
        """Resize the terminal"""
        if self.master_fd and self.running:
            try:
                import fcntl
                import termios
                import struct
                winsize = struct.pack('HHHH', rows, cols, 0, 0)
                fcntl.ioctl(self.master_fd, termios.TIOCSWINSZ, winsize)
                return True
            except Exception as e:
                logger.error(f"Error resizing terminal {self.session_id}: {e}")
                return False
        return False
    
    def stop(self):
        """Stop the terminal session"""
        self.running = False
        
        if self.process:
            try:
                # Send SIGTERM to process group
                os.killpg(os.getpgid(self.process.pid), 15)
                self.process.wait(timeout=2)
            except:
                try:
                    # Force kill if needed
                    os.killpg(os.getpgid(self.process.pid), 9)
                except:
                    pass
        
        if self.master_fd:
            try:
                os.close(self.master_fd)
            except:
                pass
        
        logger.info(f"Terminal session {self.session_id} stopped")
    
    def get_info(self) -> Dict[str, Any]:
        """Get session information"""
        return {
            'session_id': self.session_id,
            'shell': self.shell,
            'running': self.running,
            'pid': self.process.pid if self.process else None,
            'created_at': self.created_at,
            'last_activity': self.last_activity,
            'uptime': time.time() - self.created_at
        }

class TerminalService:
    """Service layer for terminal management"""
    
    def __init__(self):
        self.sessions: Dict[str, TerminalSession] = {}
        self.cleanup_interval = 300  # 5 minutes
        self.max_idle_time = 1800  # 30 minutes
        self._start_cleanup_thread()
    
    async def create_session(self, shell: str = None) -> Optional[Dict[str, Any]]:
        """Create a new terminal session
        
        Args:
            shell: Shell to use (defaults to system shell)
            
        Returns:
            Session info dict if successful, None otherwise
        """
        session_id = str(uuid.uuid4())
        session = TerminalSession(session_id, shell)
        
        if session.start():
            self.sessions[session_id] = session
            logger.info(f"Created terminal session: {session_id}")
            return session.get_info()
        else:
            return None
    
    async def get_session_info(self, session_id: str) -> Optional[Dict[str, Any]]:
        """Get session information
        
        Args:
            session_id: Session ID
            
        Returns:
            Session info dict if found, None otherwise
        """
        session = self.sessions.get(session_id)
        if session:
            return session.get_info()
        return None
    
    async def list_sessions(self) -> List[Dict[str, Any]]:
        """List all active sessions
        
        Returns:
            List of session info dicts
        """
        return [session.get_info() for session in self.sessions.values()]
    
    async def write_to_session(self, session_id: str, data: str) -> bool:
        """Write data to a terminal session
        
        Args:
            session_id: Session ID
            data: Data to write
            
        Returns:
            True if successful, False otherwise
        """
        session = self.sessions.get(session_id)
        if session:
            return session.write(data)
        return False
    
    async def read_from_session(self, session_id: str, timeout: float = 0.1) -> Optional[str]:
        """Read data from a terminal session
        
        Args:
            session_id: Session ID
            timeout: Read timeout in seconds
            
        Returns:
            Output string if available, None otherwise
        """
        session = self.sessions.get(session_id)
        if session:
            return session.read(timeout)
        return None
    
    async def resize_session(self, session_id: str, cols: int, rows: int) -> bool:
        """Resize a terminal session
        
        Args:
            session_id: Session ID
            cols: Number of columns
            rows: Number of rows
            
        Returns:
            True if successful, False otherwise
        """
        session = self.sessions.get(session_id)
        if session:
            return session.resize(cols, rows)
        return False
    
    async def remove_session(self, session_id: str) -> bool:
        """Remove and cleanup a session
        
        Args:
            session_id: Session ID
            
        Returns:
            True if session was removed, False if not found
        """
        if session_id in self.sessions:
            self.sessions[session_id].stop()
            del self.sessions[session_id]
            logger.info(f"Removed terminal session: {session_id}")
            return True
        return False
    
    def get_session_for_websocket(self, session_id: str) -> Optional[TerminalSession]:
        """Get raw session object for WebSocket handling
        
        This is used internally for WebSocket communication.
        
        Args:
            session_id: Session ID
            
        Returns:
            TerminalSession object or None
        """
        return self.sessions.get(session_id)
    
    def cleanup_dead_sessions(self):
        """Remove sessions that are no longer running or idle"""
        current_time = time.time()
        dead_sessions = []
        
        for session_id, session in self.sessions.items():
            # Check if session is dead
            if not session.running or (session.process and session.process.poll() is not None):
                dead_sessions.append(session_id)
            # Check if session is idle
            elif current_time - session.last_activity > self.max_idle_time:
                logger.info(f"Session {session_id} idle for too long")
                dead_sessions.append(session_id)
        
        for session_id in dead_sessions:
            logger.info(f"Cleaning up session: {session_id}")
            self.sessions[session_id].stop()
            del self.sessions[session_id]
    
    def _start_cleanup_thread(self):
        """Start background thread for cleanup"""
        def cleanup_loop():
            while True:
                time.sleep(self.cleanup_interval)
                self.cleanup_dead_sessions()
        
        thread = threading.Thread(target=cleanup_loop, daemon=True)
        thread.start()
        logger.info("Started terminal cleanup thread")

# Global service instance (singleton)
_terminal_service = None

def get_terminal_service() -> TerminalService:
    """Get the terminal service singleton"""
    global _terminal_service
    if _terminal_service is None:
        _terminal_service = TerminalService()
    return _terminal_service