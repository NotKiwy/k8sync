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

	deploys, err := clientset.AppsV1().Deployments(*namespace).List(context.TODO(), metav1.ListOptions{})
	if err != nil {
		fmt.Fprintf(os.Stderr, "[!] Failed to list deployments: %v\n", err)
		os.Exit(1)
	}

	normalized := normalize(deploys)

	output, err := json.MarshalIndent(normalized, "", "  ")
	if err != nil {
		fmt.Fprintf(os.Stderr, "[!] Failed to marshal JSON: %v\n", err)
		os.Exit(1)
	}

	fmt.Println(string(output))
}

func normalize(list interface{}) map[string]interface{} {
	raw, err := json.Marshal(list)
	if err != nil {
		return nil
	}

	var result map[string]interface{}
	if err := json.Unmarshal(raw, &result); err != nil {
		return nil
	}

	cleanmetadata(result)
	return result
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
