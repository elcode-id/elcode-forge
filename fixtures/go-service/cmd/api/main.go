package main

import (
	"log"

	"github.com/gin-gonic/gin"
	"example.com/goservice/internal/handlers"
)

func main() {
	r := gin.Default()
	userHandler := handlers.NewUserHandler()
	r.GET("/users/:id", userHandler.GetUser)

	if err := r.Run(":8080"); err != nil {
		log.Fatal(err)
	}
}
