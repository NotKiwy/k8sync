# k8sync

WARNING: This project is currently under active development and doesn't work yet. Code is being written in public. Follow commits to see progress.

Kubernetes drift detector for comparing cluster configurations across environments.

The problem: you deploy to dev, test on staging, everything works. Push to prod and it breaks. Turns out some ConfigMap or resource limit differs between
environments. This tool catches that.

Built with Go and Rust. Uses gRPC for communication between components.

Currently implemented: basic Go collector that reads K8s resources and Rust CLI skeleton with command parsing.

Pull requests and contributions welcome.

Apache 2.0 licensed.