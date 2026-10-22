from pydantic import BaseModel

class User(BaseModel):
    id: int
    username: str
    is_active: bool = True
