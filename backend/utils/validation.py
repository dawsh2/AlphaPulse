"""
API Validation Utilities
Pydantic integration with Flask routes for automatic validation
"""
from functools import wraps
from flask import request, jsonify, make_response
from pydantic import BaseModel, ValidationError
from typing import Type, Any, Dict, Optional
import traceback


class ValidationErrorResponse(BaseModel):
    """Standard validation error response"""
    error: str = "Validation Error"
    details: Dict[str, Any]
    status_code: int = 400


class ApiError(Exception):
    """Custom API exception with status code"""
    def __init__(self, message: str, status_code: int = 400, details: Optional[Dict] = None):
        self.message = message
        self.status_code = status_code
        self.details = details or {}
        super().__init__(message)


def validate_json(schema: Type[BaseModel]):
    """
    Decorator to validate JSON request body against Pydantic schema
    
    Usage:
        @validate_json(QueryRequest)
        def my_route(body: QueryRequest):
            # body is guaranteed to be valid QueryRequest instance
            pass
    """
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            try:
                # Get JSON data from request
                if not request.is_json:
                    return make_response(jsonify({
                        'error': 'Content-Type must be application/json',
                        'status_code': 400
                    }), 400)
                
                json_data = request.get_json()
                if json_data is None:
                    return make_response(jsonify({
                        'error': 'Request body must contain valid JSON',
                        'status_code': 400
                    }), 400)
                
                # Validate against schema
                validated_data = schema.model_validate(json_data)
                
                # Call the original function with validated data
                return func(validated_data, *args, **kwargs)
                
            except ValidationError as e:
                # Format Pydantic validation errors
                error_details = {}
                for error in e.errors():
                    field_path = ' → '.join(str(loc) for loc in error['loc'])
                    error_details[field_path] = {
                        'message': error['msg'],
                        'type': error['type'],
                        'input': error.get('input')
                    }
                
                return make_response(jsonify({
                    'error': 'Request validation failed',
                    'details': error_details,
                    'status_code': 400
                }), 400)
                
            except Exception as e:
                # Log unexpected errors
                print(f"❌ Validation error in {func.__name__}: {str(e)}")
                print(traceback.format_exc())
                
                return make_response(jsonify({
                    'error': 'Internal validation error',
                    'message': str(e),
                    'status_code': 500
                }), 500)
        
        return wrapper
    return decorator


def validate_query_params(schema: Type[BaseModel]):
    """
    Decorator to validate URL query parameters against Pydantic schema
    
    Usage:
        @validate_query_params(MarketDataRequest)
        def my_route(params: MarketDataRequest):
            # params is guaranteed to be valid MarketDataRequest instance
            pass
    """
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            try:
                # Convert query args to dict
                query_data = request.args.to_dict()
                
                # Convert string values to appropriate types based on schema
                # This is a simple conversion - more sophisticated parsing may be needed
                for field_name, field_info in schema.model_fields.items():
                    if field_name in query_data:
                        field_type = field_info.annotation
                        try:
                            if field_type == int:
                                query_data[field_name] = int(query_data[field_name])
                            elif field_type == float:
                                query_data[field_name] = float(query_data[field_name])
                            elif field_type == bool:
                                query_data[field_name] = query_data[field_name].lower() in ('true', '1', 'yes', 'on')
                        except (ValueError, TypeError):
                            pass  # Keep as string, let Pydantic handle the error
                
                # Validate against schema
                validated_params = schema.model_validate(query_data)
                
                # Call the original function with validated params
                return func(validated_params, *args, **kwargs)
                
            except ValidationError as e:
                # Format validation errors
                error_details = {}
                for error in e.errors():
                    field_path = ' → '.join(str(loc) for loc in error['loc'])
                    error_details[field_path] = {
                        'message': error['msg'],
                        'type': error['type'],
                        'input': error.get('input')
                    }
                
                return make_response(jsonify({
                    'error': 'Query parameter validation failed',
                    'details': error_details,
                    'status_code': 400
                }), 400)
                
            except Exception as e:
                print(f"❌ Query validation error in {func.__name__}: {str(e)}")
                return make_response(jsonify({
                    'error': 'Internal validation error',
                    'message': str(e),
                    'status_code': 500
                }), 500)
        
        return wrapper
    return decorator


def validate_response(schema: Type[BaseModel]):
    """
    Decorator to validate response data against Pydantic schema
    
    Usage:
        @validate_response(ApiResponse)
        def my_route():
            return {'status': 'success', 'data': {...}}
    """
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            try:
                # Call the original function
                result = func(*args, **kwargs)
                
                # Handle different return types
                if isinstance(result, tuple):
                    response_data, status_code = result
                else:
                    response_data = result
                    status_code = 200
                
                # If it's already a Flask Response, return as-is
                if hasattr(response_data, 'headers'):
                    return result
                
                # Validate response data
                if isinstance(response_data, dict):
                    validated_response = schema.model_validate(response_data)
                    return jsonify(validated_response.model_dump()), status_code
                else:
                    # For non-dict responses, wrap in ApiResponse format
                    wrapped_response = {
                        'status': 'success',
                        'data': response_data
                    }
                    validated_response = schema.model_validate(wrapped_response)
                    return jsonify(validated_response.model_dump()), status_code
                    
            except ValidationError as e:
                print(f"❌ Response validation error in {func.__name__}: {str(e)}")
                print(f"Response data: {response_data}")
                
                # Return the original response if validation fails (for debugging)
                # In production, you might want to return a generic error
                return make_response(jsonify({
                    'error': 'Response validation failed',
                    'details': str(e),
                    'original_data': response_data,
                    'status_code': 500
                }), 500)
                
            except Exception as e:
                print(f"❌ Response wrapper error in {func.__name__}: {str(e)}")
                return make_response(jsonify({
                    'error': 'Internal response validation error',
                    'message': str(e),
                    'status_code': 500
                }), 500)
        
        return wrapper
    return decorator


def handle_api_error(error: ApiError):
    """Handle custom API errors consistently"""
    return make_response(jsonify({
        'error': error.message,
        'details': error.details,
        'status_code': error.status_code
    }), error.status_code)


def add_cors_headers(response, origin='*'):
    """Add CORS headers to response"""
    response.headers.add('Access-Control-Allow-Origin', origin)
    response.headers.add('Access-Control-Allow-Headers', 'Content-Type, Authorization')
    response.headers.add('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS')
    return response