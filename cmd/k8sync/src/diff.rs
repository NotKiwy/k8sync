use serde_json::Value;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum DiffType {
    Added,
    Removed,
    Modified,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ResourceRef {
    pub restype: String,
    pub name: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Difference {
    pub restype: String,
    pub resource: String,
    pub path: String,
    pub kind: DiffType,
    pub leftval: Option<String>,
    pub rightval: Option<String>,
}

#[allow(dead_code)]
pub struct DiffResult {
    pub diffs: Vec<Difference>,
    pub leftonly: Vec<ResourceRef>,
    pub rightonly: Vec<ResourceRef>,
}

fn display_restype(restype: &str) -> &str {
    match restype {
        "deployments" => "Deployment",
        "statefulsets" => "StatefulSet",
        "services" => "Service",
        "configmaps" => "ConfigMap",
        _ => restype,
    }
}

#[allow(dead_code)]
impl DiffResult {
    pub fn is_empty(&self) -> bool {
        self.diffs.is_empty() && self.leftonly.is_empty() && self.rightonly.is_empty()
    }

    pub fn print(&self, _left: &str, _right: &str) {
        if self.is_empty() {
            println!("[+] No differences found");
            return;
        }

        println!();

        if !self.leftonly.is_empty() {
            println!("[+] Only in {}:", _left);
            let last = self.leftonly.len() - 1;
            for (i, r) in self.leftonly.iter().enumerate() {
                let branch = if i == last { "└──" } else { "├──" };
                println!("    {} {}/{}", branch, display_restype(&r.restype), r.name);
            }
            println!();
        }

        if !self.rightonly.is_empty() {
            println!("[-] Only in {}:", _right);
            let last = self.rightonly.len() - 1;
            for (i, r) in self.rightonly.iter().enumerate() {
                let branch = if i == last { "└──" } else { "├──" };
                println!("    {} {}/{}", branch, display_restype(&r.restype), r.name);
            }
            println!();
        }

        if !self.diffs.is_empty() {
            println!("[~] Modified resources:");

            let mut byresource: HashMap<String, Vec<&Difference>> = HashMap::new();
            for diff in &self.diffs {
                let key = format!("{}/{}", display_restype(&diff.restype), diff.resource);
                byresource.entry(key).or_default().push(diff);
            }

            for (resource, diffs) in byresource {
                println!("  {}:", resource);
                let last = diffs.len() - 1;
                for (i, diff) in diffs.iter().enumerate() {
                    let branch = if i == last { "└──" } else { "├──" };
                    match &diff.kind {
                        DiffType::Modified => {
                            println!(
                                "    {} {} {} -> {}",
                                branch,
                                diff.path,
                                diff.leftval.as_deref().unwrap_or("null"),
                                diff.rightval.as_deref().unwrap_or("null")
                            );
                        }
                        DiffType::Added => {
                            println!(
                                "    {} {} added: {}",
                                branch,
                                diff.path,
                                diff.rightval.as_deref().unwrap_or("null")
                            );
                        }
                        DiffType::Removed => {
                            println!(
                                "    {} {} removed: {}",
                                branch,
                                diff.path,
                                diff.leftval.as_deref().unwrap_or("null")
                            );
                        }
                    }
                }
                println!();
            }
        }
    }
}

#[allow(dead_code)]
pub fn compare_resources(left: &Value, right: &Value) -> DiffResult {
    let mut result = DiffResult {
        diffs: Vec::new(),
        leftonly: Vec::new(),
        rightonly: Vec::new(),
    };
    let _empty = vec![];

    for restype in ["deployments", "statefulsets", "services", "configmaps"] {
        let leftitems = left[restype].as_array().unwrap_or(&_empty);
        let rightitems = right[restype].as_array().unwrap_or(&_empty);
        compare_kind(restype, leftitems, rightitems, &mut result);
    }

    result
}

fn compare_kind(restype: &str, leftitems: &[Value], rightitems: &[Value], result: &mut DiffResult) {
    let mut _leftmap: HashMap<String, &Value> = HashMap::new();
    let mut _rightmap: HashMap<String, &Value> = HashMap::new();

    for item in leftitems {
        if let Some(name) = item["metadata"]["name"].as_str() {
            _leftmap.insert(name.to_string(), item);
        }
    }
    for item in rightitems {
        if let Some(name) = item["metadata"]["name"].as_str() {
            _rightmap.insert(name.to_string(), item);
        }
    }

    for name in _leftmap.keys() {
        if !_rightmap.contains_key(name) {
            result.leftonly.push(ResourceRef {
                restype: restype.to_string(),
                name: name.clone(),
            });
        }
    }
    for name in _rightmap.keys() {
        if !_leftmap.contains_key(name) {
            result.rightonly.push(ResourceRef {
                restype: restype.to_string(),
                name: name.clone(),
            });
        }
    }

    for (name, leftitem) in &_leftmap {
        if let Some(rightitem) = _rightmap.get(name) {
            let fielddiffs = compare_fields(restype, name, leftitem, rightitem);
            result.diffs.extend(fielddiffs);
        }
    }
}

fn compare_fields(restype: &str, name: &str, left: &Value, right: &Value) -> Vec<Difference> {
    match restype {
        "deployments" | "statefulsets" => compare_workload_fields(restype, name, left, right),
        "services" => compare_service_fields(name, left, right),
        "configmaps" => compare_configmap_fields(name, left, right),
        _ => Vec::new(),
    }
}

fn compare_workload_fields(
    restype: &str,
    name: &str,
    left: &Value,
    right: &Value,
) -> Vec<Difference> {
    let mut diffs = Vec::new();

    let leftreplicas = left["spec"]["replicas"].as_i64();
    let rightreplicas = right["spec"]["replicas"].as_i64();
    if leftreplicas != rightreplicas {
        diffs.push(Difference {
            restype: restype.to_string(),
            resource: name.to_string(),
            path: "spec.replicas".to_string(),
            kind: DiffType::Modified,
            leftval: leftreplicas.map(|v| v.to_string()),
            rightval: rightreplicas.map(|v| v.to_string()),
        });
    }

    if let (Some(_leftc), Some(_rightc)) = (
        left["spec"]["template"]["spec"]["containers"].as_array(),
        right["spec"]["template"]["spec"]["containers"].as_array(),
    ) {
        for (i, (lc, rc)) in _leftc.iter().zip(_rightc.iter()).enumerate() {
            let leftimg = lc["image"].as_str();
            let rightimg = rc["image"].as_str();
            if leftimg != rightimg {
                diffs.push(Difference {
                    restype: restype.to_string(),
                    resource: name.to_string(),
                    path: format!("spec.template.spec.containers[{}].image", i),
                    kind: DiffType::Modified,
                    leftval: leftimg.map(String::from),
                    rightval: rightimg.map(String::from),
                });
            }
        }
    }

    compare_labels(restype, name, left, right, &mut diffs);
    diffs
}

fn compare_service_fields(name: &str, left: &Value, right: &Value) -> Vec<Difference> {
    let mut diffs = Vec::new();

    let lefttype = left["spec"]["type"].as_str();
    let righttype = right["spec"]["type"].as_str();
    if lefttype != righttype {
        diffs.push(Difference {
            restype: "services".to_string(),
            resource: name.to_string(),
            path: "spec.type".to_string(),
            kind: DiffType::Modified,
            leftval: lefttype.map(String::from),
            rightval: righttype.map(String::from),
        });
    }

    if let (Some(_leftports), Some(_rightports)) = (
        left["spec"]["ports"].as_array(),
        right["spec"]["ports"].as_array(),
    ) {
        for (i, (lp, rp)) in _leftports.iter().zip(_rightports.iter()).enumerate() {
            let leftport = lp["port"].as_i64();
            let rightport = rp["port"].as_i64();
            if leftport != rightport {
                diffs.push(Difference {
                    restype: "services".to_string(),
                    resource: name.to_string(),
                    path: format!("spec.ports[{}].port", i),
                    kind: DiffType::Modified,
                    leftval: leftport.map(|v| v.to_string()),
                    rightval: rightport.map(|v| v.to_string()),
                });
            }
            let lefttarget = lp["targetPort"]
                .as_i64()
                .map(|v| v.to_string())
                .or_else(|| lp["targetPort"].as_str().map(String::from));
            let righttarget = rp["targetPort"]
                .as_i64()
                .map(|v| v.to_string())
                .or_else(|| rp["targetPort"].as_str().map(String::from));
            if lefttarget != righttarget {
                diffs.push(Difference {
                    restype: "services".to_string(),
                    resource: name.to_string(),
                    path: format!("spec.ports[{}].targetPort", i),
                    kind: DiffType::Modified,
                    leftval: lefttarget,
                    rightval: righttarget,
                });
            }
        }
    }

    compare_labels("services", name, left, right, &mut diffs);
    diffs
}

fn compare_configmap_fields(name: &str, left: &Value, right: &Value) -> Vec<Difference> {
    let mut diffs = Vec::new();

    if let (Some(_leftdata), Some(_rightdata)) =
        (left["data"].as_object(), right["data"].as_object())
    {
        for (key, leftval) in _leftdata {
            match _rightdata.get(key) {
                Some(rightval) if rightval != leftval => {
                    diffs.push(Difference {
                        restype: "configmaps".to_string(),
                        resource: name.to_string(),
                        path: format!("data.{}", key),
                        kind: DiffType::Modified,
                        leftval: leftval.as_str().map(String::from),
                        rightval: rightval.as_str().map(String::from),
                    });
                }
                None => {
                    diffs.push(Difference {
                        restype: "configmaps".to_string(),
                        resource: name.to_string(),
                        path: format!("data.{}", key),
                        kind: DiffType::Removed,
                        leftval: leftval.as_str().map(String::from),
                        rightval: None,
                    });
                }
                _ => {}
            }
        }
        for key in _rightdata.keys() {
            if !_leftdata.contains_key(key) {
                diffs.push(Difference {
                    restype: "configmaps".to_string(),
                    resource: name.to_string(),
                    path: format!("data.{}", key),
                    kind: DiffType::Added,
                    leftval: None,
                    rightval: _rightdata[key].as_str().map(String::from),
                });
            }
        }
    }

    compare_labels("configmaps", name, left, right, &mut diffs);
    diffs
}

fn compare_labels(
    restype: &str,
    name: &str,
    left: &Value,
    right: &Value,
    diffs: &mut Vec<Difference>,
) {
    if let (Some(_leftlabels), Some(_rightlabels)) = (
        left["metadata"]["labels"].as_object(),
        right["metadata"]["labels"].as_object(),
    ) {
        for (key, leftval) in _leftlabels {
            match _rightlabels.get(key) {
                Some(rightval) if rightval != leftval => {
                    diffs.push(Difference {
                        restype: restype.to_string(),
                        resource: name.to_string(),
                        path: format!("metadata.labels.{}", key),
                        kind: DiffType::Modified,
                        leftval: leftval.as_str().map(String::from),
                        rightval: rightval.as_str().map(String::from),
                    });
                }
                None => {
                    diffs.push(Difference {
                        restype: restype.to_string(),
                        resource: name.to_string(),
                        path: format!("metadata.labels.{}", key),
                        kind: DiffType::Removed,
                        leftval: leftval.as_str().map(String::from),
                        rightval: None,
                    });
                }
                _ => {}
            }
        }
        for key in _rightlabels.keys() {
            if !_leftlabels.contains_key(key) {
                diffs.push(Difference {
                    restype: restype.to_string(),
                    resource: name.to_string(),
                    path: format!("metadata.labels.{}", key),
                    kind: DiffType::Added,
                    leftval: None,
                    rightval: _rightlabels[key].as_str().map(String::from),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mksnap(deployments: serde_json::Value) -> serde_json::Value {
        json!({
            "deployments": deployments,
            "statefulsets": [],
            "services": [],
            "configmaps": []
        })
    }

    #[test]
    fn test_no_differences() {
        let data = mksnap(json!([{
            "metadata": {"name": "test"},
            "spec": {"replicas": 3}
        }]));
        let result = compare_resources(&data, &data);
        assert!(result.is_empty());
    }

    #[test]
    fn test_replica_difference() {
        let left = mksnap(json!([{
            "metadata": {"name": "test"},
            "spec": {"replicas": 2}
        }]));
        let right = mksnap(json!([{
            "metadata": {"name": "test"},
            "spec": {"replicas": 5}
        }]));
        let result = compare_resources(&left, &right);
        assert_eq!(result.diffs.len(), 1);
        assert_eq!(result.diffs[0].path, "spec.replicas");
    }

    #[test]
    fn test_resource_only_in_left() {
        let left = mksnap(json!([{
            "metadata": {"name": "test"},
            "spec": {"replicas": 2}
        }]));
        let right = mksnap(json!([]));
        let result = compare_resources(&left, &right);
        assert_eq!(result.leftonly.len(), 1);
        assert_eq!(result.leftonly[0].name, "test");
    }
}
