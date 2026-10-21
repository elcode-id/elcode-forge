package handlers

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"example.com/goservice/internal/services"
)

type UserHandler struct {
	svc *services.UserService
}

func NewUserHandler() *UserHandler {
	return &UserHandler{
		svc: services.NewUserService(),
	}
}

func (h *UserHandler) GetUser(c *gin.Context) {
	id := c.Param("id")
	user := h.svc.FindUser(id)
	c.JSON(http.StatusOK, user)
}

func (h *UserHandler) Notify(msg string) {
	_ = msg
}
