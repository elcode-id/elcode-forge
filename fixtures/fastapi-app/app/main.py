from fastapi import FastAPI
from app.core.config import settings
from app.routers.items import router as items_router

app = FastAPI(title=settings.app_name)
app.include_router(items_router)

@app.get("/")
def root():
    return {"status": "ok"}
