package collector

import (
	"context"
	"encoding/json"
	"fmt"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/kubernetes"
)

type Snapshot struct {
	Deployments  []interface{} `json:"deployments"`
	StatefulSets []interface{} `json:"statefulsets"`
	Services     []interface{} `json:"services"`
	ConfigMaps   []interface{} `json:"configmaps"`
}

func Collect(clientset *kubernetes.Clientset, namespace string) (*Snapshot, error) {
	deploys, err := clientset.AppsV1().Deployments(namespace).List(context.TODO(), metav1.ListOptions{})
	if err != nil {
		return nil, fmt.Errorf("deployments: %w", err)
	}

	statefulsets, err := clientset.AppsV1().StatefulSets(namespace).List(context.TODO(), metav1.ListOptions{})
	if err != nil {
		return nil, fmt.Errorf("statefulsets: %w", err)
	}

	services, err := clientset.CoreV1().Services(namespace).List(context.TODO(), metav1.ListOptions{})
	if err != nil {
		return nil, fmt.Errorf("services: %w", err)
	}

	configmaps, err := clientset.CoreV1().ConfigMaps(namespace).List(context.TODO(), metav1.ListOptions{})
	if err != nil {
		return nil, fmt.Errorf("configmaps: %w", err)
	}

	return &Snapshot{
		Deployments:  NormalizeList(deploys),
		StatefulSets: NormalizeList(statefulsets),
		Services:     NormalizeList(services),
		ConfigMaps:   filterSystemConfigMaps(NormalizeList(configmaps)),
	}, nil
}

// systemConfigMaps are auto-created by Kubernetes in every namespace and always
// differ between clusters — including them causes false positives.
var systemConfigMaps = map[string]bool{
	"kube-root-ca.crt": true,
}

func filterSystemConfigMaps(items []interface{}) []interface{} {
	filtered := items[:0]
	for _, item := range items {
		m, ok := item.(map[string]interface{})
		if !ok {
			continue
		}
		meta, ok := m["metadata"].(map[string]interface{})
		if !ok {
			filtered = append(filtered, item)
			continue
		}
		name, _ := meta["name"].(string)
		if !systemConfigMaps[name] {
			filtered = append(filtered, item)
		}
	}
	return filtered
}

func NormalizeList(list interface{}) []interface{} {
	raw, err := json.Marshal(list)
	if err != nil {
		return nil
	}
	var result map[string]interface{}
	if err := json.Unmarshal(raw, &result); err != nil {
		return nil
	}
	items, ok := result["items"].([]interface{})
	if !ok {
		return []interface{}{}
	}
	for _, item := range items {
		CleanMetadata(item)
	}
	return items
}

func CleanMetadata(data interface{}) {
	switch v := data.(type) {
	case map[string]interface{}:
		delete(v, "resourceVersion")
		delete(v, "creationTimestamp")
		delete(v, "uid")
		delete(v, "selfLink")
		delete(v, "managedFields")
		delete(v, "status")
		delete(v, "generation")
		for _, val := range v {
			CleanMetadata(val)
		}
	case []interface{}:
		for _, item := range v {
			CleanMetadata(item)
		}
	}
}
