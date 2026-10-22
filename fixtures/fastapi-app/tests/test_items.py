from app.models.user import User

def test_user_model_creation():
    user = User(id=1, username="alice", is_active=True)
    assert user.id == 1
    assert user.username == "alice"
    assert user.is_active is True
