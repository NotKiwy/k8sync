package collector

import (
	"testing"
)

func TestCleanMetadataRemovesFields(t *testing.T) {
	data := map[string]interface{}{
		"metadata": map[string]interface{}{
			"name":              "test",
			"uid":               "abc123",
			"resourceVersion":   "12345",
			"creationTimestamp": "2024-01-01",
			"selfLink":          "/api/v1/test",
			"managedFields":     []interface{}{},
			"generation":        1,
		},
		"status": map[string]interface{}{
			"ready": true,
		},
		"spec": map[string]interface{}{
			"replicas": 3,
		},
	}

	CleanMetadata(data)

	meta := data["metadata"].(map[string]interface{})
	for _, field := range []string{"uid", "resourceVersion", "creationTimestamp", "selfLink", "managedFields", "generation"} {
		if _, ok := meta[field]; ok {
			t.Errorf("field %q should be removed from metadata", field)
		}
	}
	if _, ok := data["status"]; ok {
		t.Error("status should be removed")
	}
	if meta["name"] != "test" {
		t.Error("name should be preserved")
	}
	if data["spec"] == nil {
		t.Error("spec should be preserved")
	}
}

func TestCleanMetadataRecursive(t *testing.T) {
	data := map[string]interface{}{
		"spec": map[string]interface{}{
			"template": map[string]interface{}{
				"metadata": map[string]interface{}{
					"uid":  "inner-uid",
					"name": "pod-template",
				},
			},
		},
	}

	CleanMetadata(data)

	inner := data["spec"].(map[string]interface{})["template"].(map[string]interface{})["metadata"].(map[string]interface{})
	if _, ok := inner["uid"]; ok {
		t.Error("uid should be removed from nested metadata")
	}
	if inner["name"] != "pod-template" {
		t.Error("name should be preserved in nested metadata")
	}
}

func TestCleanMetadataArray(t *testing.T) {
	data := map[string]interface{}{
		"items": []interface{}{
			map[string]interface{}{
				"uid":  "abc",
				"name": "item1",
			},
			map[string]interface{}{
				"uid":  "def",
				"name": "item2",
			},
		},
	}

	CleanMetadata(data)

	items := data["items"].([]interface{})
	for i, item := range items {
		m := item.(map[string]interface{})
		if _, ok := m["uid"]; ok {
			t.Errorf("uid should be removed from array item %d", i)
		}
	}
}

func TestCleanMetadataPreservesData(t *testing.T) {
	data := map[string]interface{}{
		"data": map[string]interface{}{
			"config.yaml": "key: value",
			"LOG_LEVEL":   "debug",
		},
		"spec": map[string]interface{}{
			"replicas": 5,
		},
	}

	CleanMetadata(data)

	d := data["data"].(map[string]interface{})
	if d["config.yaml"] != "key: value" {
		t.Error("data.config.yaml should be preserved")
	}
	if d["LOG_LEVEL"] != "debug" {
		t.Error("data.LOG_LEVEL should be preserved")
	}
	if data["spec"].(map[string]interface{})["replicas"] != 5 {
		t.Error("spec.replicas should be preserved")
	}
}

func TestCleanMetadataAllSeven(t *testing.T) {
	_fields := []string{"resourceVersion", "creationTimestamp", "uid", "selfLink", "managedFields", "status", "generation"}
	data := map[string]interface{}{}
	for _, f := range _fields {
		data[f] = "value"
	}
	data["name"] = "keep"

	CleanMetadata(data)

	for _, f := range _fields {
		if _, ok := data[f]; ok {
			t.Errorf("%q should be removed", f)
		}
	}
	if data["name"] != "keep" {
		t.Error("name should be preserved")
	}
}

func TestNormalizeListExtractsItems(t *testing.T) {
	type fakeitem struct {
		Metadata map[string]interface{} `json:"metadata"`
		Spec     map[string]interface{} `json:"spec"`
	}
	type fakelist struct {
		Items []fakeitem `json:"items"`
	}

	list := fakelist{
		Items: []fakeitem{
			{
				Metadata: map[string]interface{}{"name": "nginx", "uid": "abc"},
				Spec:     map[string]interface{}{"replicas": 3},
			},
			{
				Metadata: map[string]interface{}{"name": "redis", "uid": "def"},
				Spec:     map[string]interface{}{"replicas": 1},
			},
		},
	}

	result := NormalizeList(list)

	if len(result) != 2 {
		t.Fatalf("expected 2 items, got %d", len(result))
	}
}

func TestNormalizeListCleansMetadata(t *testing.T) {
	type fakeitem struct {
		Metadata map[string]interface{} `json:"metadata"`
	}
	type fakelist struct {
		Items []fakeitem `json:"items"`
	}

	list := fakelist{
		Items: []fakeitem{
			{Metadata: map[string]interface{}{"name": "test", "uid": "abc", "resourceVersion": "1"}},
		},
	}

	result := NormalizeList(list)

	item := result[0].(map[string]interface{})
	meta := item["metadata"].(map[string]interface{})
	if _, ok := meta["uid"]; ok {
		t.Error("uid should be removed")
	}
	if meta["name"] != "test" {
		t.Error("name should be preserved")
	}
}

func TestNormalizeListEmpty(t *testing.T) {
	type fakelist struct {
		Items []interface{} `json:"items"`
	}

	result := NormalizeList(fakelist{Items: []interface{}{}})
	if len(result) != 0 {
		t.Errorf("expected 0 items, got %d", len(result))
	}
}

func TestNormalizeListMultipleItemsAllCleaned(t *testing.T) {
	type fakeitem struct {
		Metadata map[string]interface{} `json:"metadata"`
	}
	type fakelist struct {
		Items []fakeitem `json:"items"`
	}

	list := fakelist{
		Items: []fakeitem{
			{Metadata: map[string]interface{}{"name": "a", "uid": "1", "resourceVersion": "100"}},
			{Metadata: map[string]interface{}{"name": "b", "uid": "2", "resourceVersion": "200"}},
			{Metadata: map[string]interface{}{"name": "c", "uid": "3", "resourceVersion": "300"}},
		},
	}

	result := NormalizeList(list)
	if len(result) != 3 {
		t.Fatalf("expected 3 items, got %d", len(result))
	}

	for i, r := range result {
		meta := r.(map[string]interface{})["metadata"].(map[string]interface{})
		if _, ok := meta["uid"]; ok {
			t.Errorf("item %d: uid should be removed", i)
		}
		if _, ok := meta["resourceVersion"]; ok {
			t.Errorf("item %d: resourceVersion should be removed", i)
		}
	}
}

func TestCleanMetadataTableDriven(t *testing.T) {
	tests := []struct {
		name      string
		field     string
		preserved bool
	}{
		{"uid removed", "uid", false},
		{"resourceVersion removed", "resourceVersion", false},
		{"creationTimestamp removed", "creationTimestamp", false},
		{"selfLink removed", "selfLink", false},
		{"managedFields removed", "managedFields", false},
		{"status removed", "status", false},
		{"generation removed", "generation", false},
		{"name preserved", "name", true},
		{"spec preserved", "spec", true},
		{"data preserved", "data", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			data := map[string]interface{}{
				tt.field: "value",
			}
			CleanMetadata(data)
			_, exists := data[tt.field]
			if exists != tt.preserved {
				if tt.preserved {
					t.Errorf("field %q should be preserved but was removed", tt.field)
				} else {
					t.Errorf("field %q should be removed but was preserved", tt.field)
				}
			}
		})
	}
}
