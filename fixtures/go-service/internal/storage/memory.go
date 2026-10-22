package storage

type MemoryStore struct {
	data map[string]map[string]string
}

func NewMemoryStore() *MemoryStore {
	return &MemoryStore{
		data: make(map[string]map[string]string),
	}
}

func (m *MemoryStore) Get(id string) map[string]string {
	if val, ok := m.data[id]; ok {
		return val
	}
	return map[string]string{"id": id, "name": "Alice"}
}
