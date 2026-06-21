use k8sync::diff::compare_resources;
use serde_json::json;

fn snap(
    deployments: serde_json::Value,
    statefulsets: serde_json::Value,
    services: serde_json::Value,
    configmaps: serde_json::Value,
) -> serde_json::Value {
    json!({
        "deployments": deployments,
        "statefulsets": statefulsets,
        "services": services,
        "configmaps": configmaps
    })
}

fn deploys(items: serde_json::Value) -> serde_json::Value {
    snap(items, json!([]), json!([]), json!([]))
}

fn services(items: serde_json::Value) -> serde_json::Value {
    snap(json!([]), json!([]), items, json!([]))
}

fn configmaps(items: serde_json::Value) -> serde_json::Value {
    snap(json!([]), json!([]), json!([]), items)
}

fn statefulsets(items: serde_json::Value) -> serde_json::Value {
    snap(json!([]), items, json!([]), json!([]))
}

// --- deployments ---

#[test]
fn test_deploy_identical() {
    let data = deploys(json!([{
        "metadata": {"name": "nginx"},
        "spec": {"replicas": 3}
    }]));
    assert!(compare_resources(&data, &data).is_empty());
}

#[test]
fn test_deploy_image_change() {
    let left = deploys(json!([{
        "metadata": {"name": "nginx"},
        "spec": {"template": {"spec": {"containers": [{"image": "nginx:1.25"}]}}}
    }]));
    let right = deploys(json!([{
        "metadata": {"name": "nginx"},
        "spec": {"template": {"spec": {"containers": [{"image": "nginx:1.26"}]}}}
    }]));
    let result = compare_resources(&left, &right);
    assert_eq!(result.diffs.len(), 1);
    assert!(result.diffs[0].path.contains("image"));
}

#[test]
fn test_deploy_only_in_left() {
    let left = deploys(json!([{"metadata": {"name": "redis"}, "spec": {}}]));
    let right = deploys(json!([]));
    let result = compare_resources(&left, &right);
    assert_eq!(result.leftonly.len(), 1);
    assert_eq!(result.leftonly[0].name, "redis");
    assert_eq!(result.leftonly[0].restype, "deployments");
}

#[test]
fn test_deploy_only_in_right() {
    let left = deploys(json!([]));
    let right = deploys(json!([{"metadata": {"name": "redis"}, "spec": {}}]));
    let result = compare_resources(&left, &right);
    assert_eq!(result.rightonly.len(), 1);
    assert_eq!(result.rightonly[0].name, "redis");
}

// --- statefulsets ---

#[test]
fn test_statefulset_replicas() {
    let left = statefulsets(json!([{
        "metadata": {"name": "postgres"},
        "spec": {"replicas": 1}
    }]));
    let right = statefulsets(json!([{
        "metadata": {"name": "postgres"},
        "spec": {"replicas": 3}
    }]));
    let result = compare_resources(&left, &right);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0].path, "spec.replicas");
    assert_eq!(result.diffs[0].restype, "statefulsets");
}

// --- services ---

#[test]
fn test_service_type_change() {
    let left = services(json!([{
        "metadata": {"name": "nginx"},
        "spec": {"type": "ClusterIP", "ports": []}
    }]));
    let right = services(json!([{
        "metadata": {"name": "nginx"},
        "spec": {"type": "LoadBalancer", "ports": []}
    }]));
    let result = compare_resources(&left, &right);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0].path, "spec.type");
}

#[test]
fn test_service_port_change() {
    let left = services(json!([{
        "metadata": {"name": "nginx"},
        "spec": {"ports": [{"port": 80, "targetPort": 8080}]}
    }]));
    let right = services(json!([{
        "metadata": {"name": "nginx"},
        "spec": {"ports": [{"port": 443, "targetPort": 8080}]}
    }]));
    let result = compare_resources(&left, &right);
    assert_eq!(result.diffs.len(), 1);
    assert!(result.diffs[0].path.contains("port"));
}

#[test]
fn test_service_identical() {
    let data = services(json!([{
        "metadata": {"name": "nginx"},
        "spec": {"type": "ClusterIP", "ports": [{"port": 80}]}
    }]));
    assert!(compare_resources(&data, &data).is_empty());
}

// --- configmaps ---

#[test]
fn test_configmap_data_modified() {
    let left = configmaps(json!([{
        "metadata": {"name": "app-config"},
        "data": {"LOG_LEVEL": "debug"}
    }]));
    let right = configmaps(json!([{
        "metadata": {"name": "app-config"},
        "data": {"LOG_LEVEL": "info"}
    }]));
    let result = compare_resources(&left, &right);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0].path, "data.LOG_LEVEL");
}

#[test]
fn test_configmap_data_added() {
    let left = configmaps(json!([{
        "metadata": {"name": "app-config"},
        "data": {}
    }]));
    let right = configmaps(json!([{
        "metadata": {"name": "app-config"},
        "data": {"NEW_KEY": "value"}
    }]));
    let result = compare_resources(&left, &right);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0].path, "data.NEW_KEY");
}

#[test]
fn test_configmap_data_removed() {
    let left = configmaps(json!([{
        "metadata": {"name": "app-config"},
        "data": {"OLD_KEY": "value"}
    }]));
    let right = configmaps(json!([{
        "metadata": {"name": "app-config"},
        "data": {}
    }]));
    let result = compare_resources(&left, &right);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0].path, "data.OLD_KEY");
}

// --- labels ---

#[test]
fn test_label_modified() {
    let left = deploys(json!([{
        "metadata": {"name": "nginx", "labels": {"env": "staging"}},
        "spec": {}
    }]));
    let right = deploys(json!([{
        "metadata": {"name": "nginx", "labels": {"env": "production"}},
        "spec": {}
    }]));
    let result = compare_resources(&left, &right);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0].path, "metadata.labels.env");
}

#[test]
fn test_label_added() {
    let left = deploys(json!([{
        "metadata": {"name": "nginx", "labels": {}},
        "spec": {}
    }]));
    let right = deploys(json!([{
        "metadata": {"name": "nginx", "labels": {"team": "platform"}},
        "spec": {}
    }]));
    let result = compare_resources(&left, &right);
    assert_eq!(result.diffs.len(), 1);
    assert_eq!(result.diffs[0].path, "metadata.labels.team");
}

// --- mixed resource types ---

#[test]
fn test_multiple_resource_types() {
    let left = snap(
        json!([{"metadata": {"name": "nginx"}, "spec": {"replicas": 3}}]),
        json!([]),
        json!([{"metadata": {"name": "nginx"}, "spec": {"type": "ClusterIP", "ports": []}}]),
        json!([{"metadata": {"name": "config"}, "data": {"KEY": "old"}}]),
    );
    let right = snap(
        json!([{"metadata": {"name": "nginx"}, "spec": {"replicas": 2}}]),
        json!([]),
        json!([{"metadata": {"name": "nginx"}, "spec": {"type": "LoadBalancer", "ports": []}}]),
        json!([{"metadata": {"name": "config"}, "data": {"KEY": "new"}}]),
    );
    let result = compare_resources(&left, &right);
    assert_eq!(result.diffs.len(), 3);
    assert!(!result.is_empty());
}

#[test]
fn test_empty_clusters() {
    let data = snap(json!([]), json!([]), json!([]), json!([]));
    assert!(compare_resources(&data, &data).is_empty());
}
