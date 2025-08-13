"""
Terminal API Routes for FastAPI - WebSocket endpoints for real terminal sessions
"""
from fastapi import APIRouter, WebSocket, WebSocketDisconnect, HTTPException, Depends
from pydantic import BaseModel, Field
from typing import Optional, List, Dict, Any
import asyncio
import json
import logging

from services.terminal_service import get_terminal_service, TerminalService

# Setup logging
logger = logging.getLogger(__name__)

# Create router
router = APIRouter(
    prefix="/api/terminal",
    tags=["terminal"],
    responses={404: {"description": "Not found"}},
)

# Pydantic models
class CreateSessionRequest(BaseModel):
    """Request to create a new terminal session"""
    shell: Optional[str] = Field(None, description="Shell to use (defaults to system shell)")

class SessionInfo(BaseModel):
    """Terminal session information"""
    session_id: str
    shell: str
    running: bool
    pid: Optional[int] = None
    created_at: float
    last_activity: float
    uptime: float

class SessionListResponse(BaseModel):
    """Response for listing sessions"""
    sessions: List[SessionInfo]
    total: int

class SessionResponse(BaseModel):
    """Response for session operations"""
    success: bool
    session: Optional[SessionInfo] = None
    message: Optional[str] = None

# REST API Endpoints
@router.post("/sessions", response_model=SessionResponse)
async def create_terminal_session(
    request: CreateSessionRequest,
    service: TerminalService = Depends(get_terminal_service)
):
    """
    Create a new terminal session
    
    Args:
        request: Session creation parameters
        
    Returns:
        Session information
        
    Raises:
        HTTPException: If session creation fails
    """
    try:
        session_info = await service.create_session(request.shell)
        if session_info:
            return SessionResponse(
                success=True,
                session=SessionInfo(**session_info),
                message="Session created successfully"
            )
        else:
            raise HTTPException(status_code=500, detail="Failed to create terminal session")
    except Exception as e:
        logger.error(f"Error creating terminal session: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/sessions", response_model=SessionListResponse)
async def list_terminal_sessions(
    service: TerminalService = Depends(get_terminal_service)
):
    """
    List all active terminal sessions
    
    Returns:
        List of session information
    """
    try:
        sessions = await service.list_sessions()
        return SessionListResponse(
            sessions=[SessionInfo(**s) for s in sessions],
            total=len(sessions)
        )
    except Exception as e:
        logger.error(f"Error listing sessions: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/sessions/{session_id}", response_model=SessionInfo)
async def get_terminal_session(
    session_id: str,
    service: TerminalService = Depends(get_terminal_service)
):
    """
    Get information about a specific terminal session
    
    Args:
        session_id: Session ID
        
    Returns:
        Session information
        
    Raises:
        HTTPException: If session not found
    """
    try:
        session_info = await service.get_session_info(session_id)
        if session_info:
            return SessionInfo(**session_info)
        else:
            raise HTTPException(status_code=404, detail="Session not found")
    except Exception as e:
        logger.error(f"Error getting session {session_id}: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.delete("/sessions/{session_id}", response_model=SessionResponse)
async def delete_terminal_session(
    session_id: str,
    service: TerminalService = Depends(get_terminal_service)
):
    """
    Delete a terminal session
    
    Args:
        session_id: Session ID
        
    Returns:
        Success response
        
    Raises:
        HTTPException: If session not found
    """
    try:
        removed = await service.remove_session(session_id)
        if removed:
            return SessionResponse(
                success=True,
                message=f"Session {session_id} deleted successfully"
            )
        else:
            raise HTTPException(status_code=404, detail="Session not found")
    except Exception as e:
        logger.error(f"Error deleting session {session_id}: {e}")
        raise HTTPException(status_code=500, detail=str(e))

# WebSocket endpoint for terminal interaction
@router.websocket("/ws/{session_id}")
async def terminal_websocket(
    websocket: WebSocket,
    session_id: str,
    service: TerminalService = Depends(get_terminal_service)
):
    """
    WebSocket endpoint for terminal interaction
    
    Args:
        websocket: WebSocket connection
        session_id: Terminal session ID
        service: Terminal service instance
    """
    await websocket.accept()
    logger.info(f"WebSocket connected for session {session_id}")
    
    # Get the terminal session
    session = service.get_session_for_websocket(session_id)
    if not session:
        await websocket.send_json({
            "type": "error",
            "message": f"Session {session_id} not found"
        })
        await websocket.close()
        return
    
    # Create tasks for bidirectional communication
    async def read_terminal():
        """Read from terminal and send to client"""
        while session.running:
            try:
                # Read from terminal (non-blocking)
                output = await asyncio.get_event_loop().run_in_executor(
                    None, session.read, 0.05
                )
                if output:
                    await websocket.send_json({
                        "type": "output",
                        "data": output
                    })
                else:
                    await asyncio.sleep(0.01)
            except Exception as e:
                logger.error(f"Error reading terminal for {session_id}: {e}")
                break
    
    async def handle_client():
        """Handle messages from client"""
        try:
            while True:
                # Receive message from client
                data = await websocket.receive_json()
                msg_type = data.get("type")
                
                if msg_type == "input":
                    # Write input to terminal
                    input_data = data.get("data", "")
                    await asyncio.get_event_loop().run_in_executor(
                        None, session.write, input_data
                    )
                    
                elif msg_type == "resize":
                    # Resize terminal
                    cols = data.get("cols", 80)
                    rows = data.get("rows", 24)
                    await asyncio.get_event_loop().run_in_executor(
                        None, session.resize, cols, rows
                    )
                    
                elif msg_type == "ping":
                    # Respond to ping
                    await websocket.send_json({
                        "type": "pong"
                    })
                    
        except WebSocketDisconnect:
            logger.info(f"WebSocket disconnected for session {session_id}")
        except Exception as e:
            logger.error(f"Error handling client for {session_id}: {e}")
    
    # Run both tasks concurrently
    try:
        await asyncio.gather(
            read_terminal(),
            handle_client()
        )
    except Exception as e:
        logger.error(f"WebSocket error for session {session_id}: {e}")
    finally:
        logger.info(f"WebSocket closed for session {session_id}")

# Alternative WebSocket endpoint that creates a new session
@router.websocket("/ws")
async def terminal_websocket_new(
    websocket: WebSocket,
    service: TerminalService = Depends(get_terminal_service)
):
    """
    WebSocket endpoint that creates a new terminal session
    
    Args:
        websocket: WebSocket connection
        service: Terminal service instance
    """
    await websocket.accept()
    logger.info("New WebSocket connection for terminal")
    
    # Create a new terminal session
    session_info = await service.create_session()
    if not session_info:
        await websocket.send_json({
            "type": "error",
            "message": "Failed to create terminal session"
        })
        await websocket.close()
        return
    
    session_id = session_info["session_id"]
    
    # Send session info to client
    await websocket.send_json({
        "type": "session_created",
        "session_id": session_id,
        "session": session_info
    })
    
    # Get the terminal session
    session = service.get_session_for_websocket(session_id)
    
    # Create tasks for bidirectional communication
    async def read_terminal():
        """Read from terminal and send to client"""
        while session.running:
            try:
                # Read from terminal (non-blocking)
                output = await asyncio.get_event_loop().run_in_executor(
                    None, session.read, 0.05
                )
                if output:
                    await websocket.send_json({
                        "type": "output",
                        "data": output
                    })
                else:
                    await asyncio.sleep(0.01)
            except Exception as e:
                logger.error(f"Error reading terminal: {e}")
                break
    
    async def handle_client():
        """Handle messages from client"""
        try:
            while True:
                # Receive message from client
                data = await websocket.receive_json()
                msg_type = data.get("type")
                
                if msg_type == "input":
                    # Write input to terminal
                    input_data = data.get("data", "")
                    await asyncio.get_event_loop().run_in_executor(
                        None, session.write, input_data
                    )
                    
                elif msg_type == "resize":
                    # Resize terminal
                    cols = data.get("cols", 80)
                    rows = data.get("rows", 24)
                    await asyncio.get_event_loop().run_in_executor(
                        None, session.resize, cols, rows
                    )
                    
                elif msg_type == "ping":
                    # Respond to ping
                    await websocket.send_json({
                        "type": "pong"
                    })
                    
        except WebSocketDisconnect:
            logger.info(f"WebSocket disconnected")
        except Exception as e:
            logger.error(f"Error handling client: {e}")
    
    # Run both tasks concurrently
    try:
        await asyncio.gather(
            read_terminal(),
            handle_client()
        )
    except Exception as e:
        logger.error(f"WebSocket error: {e}")
    finally:
        # Cleanup session when WebSocket closes
        await service.remove_session(session_id)
        logger.info(f"WebSocket closed and session {session_id} cleaned up")

# Health check for terminal service
@router.get("/health")
async def terminal_health(
    service: TerminalService = Depends(get_terminal_service)
):
    """Health check for terminal service"""
    sessions = await service.list_sessions()
    
    return {
        "service": "terminal",
        "status": "healthy",
        "active_sessions": len(sessions),
        "max_idle_time": service.max_idle_time,
        "cleanup_interval": service.cleanup_interval
    }