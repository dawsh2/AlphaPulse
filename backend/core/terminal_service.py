"""
Terminal Service with PTY support
Provides real terminal sessions via WebSocket
"""
import os
import pty
import select
import subprocess
import threading
import time
import json
from typing import Dict, Optional
import uuid
import shlex

class TerminalSession:
    def __init__(self, session_id: str, shell: str = None):
        self.session_id = session_id
        self.shell = shell or os.environ.get('SHELL', '/bin/bash')
        self.master_fd = None
        self.slave_fd = None
        self.process = None
        self.running = False
        self.cwd = os.path.expanduser('~')
        
    def start(self):
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
            
            print(f"Terminal session {self.session_id} started with PID {self.process.pid}")
            return True
            
        except Exception as e:
            print(f"Failed to start terminal session {self.session_id}: {e}")
            return False
    
    def write(self, data: str):
        """Write data to the terminal"""
        if self.master_fd and self.running:
            try:
                os.write(self.master_fd, data.encode('utf-8'))
            except OSError as e:
                print(f"Error writing to terminal {self.session_id}: {e}")
                self.running = False
    
    def read(self, timeout: float = 0.1) -> str:
        """Read available data from the terminal"""
        if not self.master_fd or not self.running:
            return ""
        
        try:
            # Use select to check if data is available
            ready, _, _ = select.select([self.master_fd], [], [], timeout)
            if ready:
                data = os.read(self.master_fd, 1024)
                return data.decode('utf-8', errors='ignore')
        except OSError as e:
            print(f"Error reading from terminal {self.session_id}: {e}")
            self.running = False
        
        return ""
    
    def resize(self, cols: int, rows: int):
        """Resize the terminal"""
        if self.master_fd and self.running:
            try:
                import fcntl
                import termios
                import struct
                winsize = struct.pack('HHHH', rows, cols, 0, 0)
                fcntl.ioctl(self.master_fd, termios.TIOCSWINSZ, winsize)
            except Exception as e:
                print(f"Error resizing terminal {self.session_id}: {e}")
    
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
        
        print(f"Terminal session {self.session_id} stopped")

class TerminalManager:
    def __init__(self):
        self.sessions: Dict[str, TerminalSession] = {}
        self.cleanup_interval = 300  # 5 minutes
        self._start_cleanup_thread()
    
    def create_session(self, shell: str = None) -> str:
        """Create a new terminal session"""
        session_id = str(uuid.uuid4())
        session = TerminalSession(session_id, shell)
        
        if session.start():
            self.sessions[session_id] = session
            return session_id
        else:
            return None
    
    def get_session(self, session_id: str) -> Optional[TerminalSession]:
        """Get an existing session"""
        return self.sessions.get(session_id)
    
    def remove_session(self, session_id: str):
        """Remove and cleanup a session"""
        if session_id in self.sessions:
            self.sessions[session_id].stop()
            del self.sessions[session_id]
    
    def cleanup_dead_sessions(self):
        """Remove sessions that are no longer running"""
        dead_sessions = []
        for session_id, session in self.sessions.items():
            if not session.running or (session.process and session.process.poll() is not None):
                dead_sessions.append(session_id)
        
        for session_id in dead_sessions:
            print(f"Cleaning up dead session: {session_id}")
            self.remove_session(session_id)
    
    def _start_cleanup_thread(self):
        """Start background thread for cleanup"""
        def cleanup_loop():
            while True:
                time.sleep(self.cleanup_interval)
                self.cleanup_dead_sessions()
        
        thread = threading.Thread(target=cleanup_loop, daemon=True)
        thread.start()

# Global terminal manager instance
terminal_manager = TerminalManager()