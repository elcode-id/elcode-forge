from fastapi import APIRouter
from app.services.user_service import get_current_user

router = APIRouter(prefix="/items", tags=["items"])

@router.get("/")
def list_items():
    user = get_current_user(1)
    return {"message": "items list", "owner": user.username}
