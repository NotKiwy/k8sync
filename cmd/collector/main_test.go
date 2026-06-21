package main

import (
	"testing"
)

func TestCleanmetadataRemovesFields(t *testing.T) {
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

	cleanmetadata(data)

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

func TestCleanmetadataRecursive(t *testing.T) {
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

	cleanmetadata(data)

	inner := data["spec"].(map[string]interface{})["template"].(map[string]interface{})["metadata"].(map[string]interface{})
	if _, ok := inner["uid"]; ok {
		t.Error("uid should be removed from nested metadata")
	}
	if inner["name"] != "pod-template" {
		t.Error("name should be preserved in nested metadata")
	}
}

func TestCleanmetadataArray(t *testing.T) {
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

	cleanmetadata(data)

	items := data["items"].([]interface{})
	for _, item := range items {
		m := item.(map[string]interface{})
		if _, ok := m["uid"]; ok {
			t.Error("uid should be removed from array items")
		}
	}
}

func TestNormalizelistExtractsItems(t *testing.T) {
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

	result := normalizelist(list)

	if len(result) != 2 {
		t.Fatalf("expected 2 items, got %d", len(result))
	}
}

func TestNormalizelistCleansMetadata(t *testing.T) {
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

	result := normalizelist(list)

	item := result[0].(map[string]interface{})
	meta := item["metadata"].(map[string]interface{})
	if _, ok := meta["uid"]; ok {
		t.Error("uid should be removed")
	}
	if meta["name"] != "test" {
		t.Error("name should be preserved")
	}
}

func TestNormalizelistEmpty(t *testing.T) {
	type fakelist struct {
		Items []interface{} `json:"items"`
	}

	result := normalizelist(fakelist{Items: []interface{}{}})
	if len(result) != 0 {
		t.Errorf("expected 0 items, got %d", len(result))
	}
}
