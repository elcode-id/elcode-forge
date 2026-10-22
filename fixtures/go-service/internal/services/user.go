package services

import (
	"example.com/goservice/internal/handlers"
	"example.com/goservice/internal/storage"
)

type UserService struct {
	store *storage.MemoryStore
}

func NewUserService() *UserService {
	return &UserService{
		store: storage.NewMemoryStore(),
	}
}

func (s *UserService) FindUser(id string) map[string]string {
	// Intentional circular dependency: services calling back into handlers
	h := handlers.NewUserHandler()
	h.Notify("finding user")
	return s.store.Get(id)
}
