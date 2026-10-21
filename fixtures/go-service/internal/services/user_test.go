package services

import (
	"testing"
)

func TestFindUser(t *testing.T) {
	svc := NewUserService()
	user := svc.FindUser("123")
	if user["name"] != "Alice" {
		t.Errorf("expected Alice, got %s", user["name"])
	}
}
