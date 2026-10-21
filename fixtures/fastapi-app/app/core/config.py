from pydantic import BaseModel

class Settings(BaseModel):
    app_name: str = "FastAPI Example App"
    debug: bool = False

settings = Settings()
