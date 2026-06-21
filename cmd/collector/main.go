package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/tools/clientcmd"
	"k8s.io/client-go/util/homedir"
)

type snapshot struct {
	Deployments  []interface{} `json:"deployments"`
	StatefulSets []interface{} `json:"statefulsets"`
	Services     []interface{} `json:"services"`
	ConfigMaps   []interface{} `json:"configmaps"`
}

func main() {
	var (
		ctxname   = flag.String("context", "", "Kubernetes context name")
		namespace = flag.String("namespace", "default", "Namespace to scan")
		kubeconf  = flag.String("kubeconfig", "", "Path to kubeconfig file")
	)
	flag.Parse()

	var _kubeconfig string
	if *kubeconf != "" {
		_kubeconfig = *kubeconf
	} else if home := homedir.HomeDir(); home != "" {
		_kubeconfig = filepath.Join(home, ".kube", "config")
	} else {
		fmt.Fprintln(os.Stderr, "[!] Cannot find kubeconfig")
		os.Exit(1)
	}

	loadrules := clientcmd.NewDefaultClientConfigLoadingRules()
	loadrules.ExplicitPath = _kubeconfig

	overrides := &clientcmd.ConfigOverrides{}
	if *ctxname != "" {
		overrides.CurrentContext = *ctxname
	}

	cfg := clientcmd.NewNonInteractiveDeferredLoadingClientConfig(loadrules, overrides)

	config, err := cfg.ClientConfig()
	if err != nil {
		fmt.Fprintf(os.Stderr, "[!] Failed to load config: %v\n", err)
		os.Exit(1)
	}

	clientset, err := kubernetes.NewForConfig(config)
	if err != nil {
		fmt.Fprintf(os.Stderr, "[!] Failed to create client: %v\n", err)
		os.Exit(1)
	}

	snap, err := collect(clientset, *namespace)
	if err != nil {
		fmt.Fprintf(os.Stderr, "[!] Failed to collect resources: %v\n", err)
		os.Exit(1)
	}

	output, err := json.MarshalIndent(snap, "", "  ")
	if err != nil {
		fmt.Fprintf(os.Stderr, "[!] Failed to marshal JSON: %v\n", err)
		os.Exit(1)
	}

	fmt.Println(string(output))
}

func collect(clientset *kubernetes.Clientset, namespace string) (*snapshot, error) {
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

	return &snapshot{
		Deployments:  normalizelist(deploys),
		StatefulSets: normalizelist(statefulsets),
		Services:     normalizelist(services),
		ConfigMaps:   normalizelist(configmaps),
	}, nil
}

func normalizelist(list interface{}) []interface{} {
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
		cleanmetadata(item)
	}
	return items
}

func cleanmetadata(data interface{}) {
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
			cleanmetadata(val)
		}
	case []interface{}:
		for _, item := range v {
			cleanmetadata(item)
		}
	}
}
