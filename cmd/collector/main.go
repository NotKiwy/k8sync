package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"

	"github.com/NotKiwy/k8sync/pkg/collector"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/rest"
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

	config, err := buildConfig(*kubeconf, *ctxname)
	if err != nil {
		fmt.Fprintf(os.Stderr, "[!] Failed to load config: %v\n", err)
		os.Exit(1)
	}

	clientset, err := kubernetes.NewForConfig(config)
	if err != nil {
		fmt.Fprintf(os.Stderr, "[!] Failed to create client: %v\n", err)
		os.Exit(1)
	}

	snap, err := collector.Collect(clientset, *namespace)
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

func buildConfig(kubeconf, ctxname string) (*rest.Config, error) {
	// explicit kubeconfig path takes priority
	if kubeconf != "" {
		return buildKubeconfigConfig(kubeconf, ctxname)
	}

	// try in-cluster config when running inside a pod
	if cfg, err := rest.InClusterConfig(); err == nil {
		return cfg, nil
	}

	// fall back to ~/.kube/config
	if home := homedir.HomeDir(); home != "" {
		return buildKubeconfigConfig(filepath.Join(home, ".kube", "config"), ctxname)
	}

	return nil, fmt.Errorf("no kubeconfig found and not running in-cluster")
}

func buildKubeconfigConfig(path, ctxname string) (*rest.Config, error) {
	loadrules := clientcmd.NewDefaultClientConfigLoadingRules()
	loadrules.ExplicitPath = path

	overrides := &clientcmd.ConfigOverrides{}
	if ctxname != "" {
		overrides.CurrentContext = ctxname
	}

	return clientcmd.NewNonInteractiveDeferredLoadingClientConfig(loadrules, overrides).ClientConfig()
}
