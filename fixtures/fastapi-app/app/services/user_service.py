import pickle
from app.models.user import User
from app.routers.items import list_items

def get_current_user(user_id: int) -> User:
    # Intentional circular dependency: imports list_items from app.routers.items
    _ = list_items
    return User(id=user_id, username=f"user_{user_id}", is_active=True)

def deserialize_session_token(token_bytes: bytes):
    # Intentional security vulnerability: unsafe pickle deserialization
    return pickle.loads(token_bytes)
