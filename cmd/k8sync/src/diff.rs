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
pub struct Difference {
    pub resource: String,
    pub path: String,
    pub kind: DiffType,
    pub leftval: Option<String>,
    pub rightval: Option<String>,
}

#[allow(dead_code)]
pub struct DiffResult {
    pub diffs: Vec<Difference>,
    pub leftonly: Vec<String>,
    pub rightonly: Vec<String>,
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
            for name in &self.leftonly {
                println!("    {}", name);
            }
            println!();
        }

        if !self.rightonly.is_empty() {
            println!("[-] Only in {}:", _right);
            for name in &self.rightonly {
                println!("    {}", name);
            }
            println!();
        }

        if !self.diffs.is_empty() {
            println!("[~] Modified resources:");
            println!();

            let mut byresource: HashMap<String, Vec<&Difference>> = HashMap::new();
            for diff in &self.diffs {
                byresource
                    .entry(diff.resource.clone())
                    .or_default()
                    .push(diff);
            }

            for (resource, diffs) in byresource {
                println!("  {}:", resource);
                for diff in diffs {
                    match &diff.kind {
                        DiffType::Modified => {
                            println!(
                                "    {} {} -> {}",
                                diff.path,
                                diff.leftval.as_deref().unwrap_or("null"),
                                diff.rightval.as_deref().unwrap_or("null")
                            );
                        }
                        DiffType::Added => {
                            println!(
                                "    {} added: {}",
                                diff.path,
                                diff.rightval.as_deref().unwrap_or("null")
                            );
                        }
                        DiffType::Removed => {
                            println!(
                                "    {} removed: {}",
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
pub fn compare_deployments(left: &Value, right: &Value) -> DiffResult {
    let mut result = DiffResult {
        diffs: Vec::new(),
        leftonly: Vec::new(),
        rightonly: Vec::new(),
    };

    let _empty = vec![];
    let leftitems = left["items"].as_array().unwrap_or(&_empty);
    let rightitems = right["items"].as_array().unwrap_or(&_empty);

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
            result.leftonly.push(name.clone());
        }
    }

    for name in _rightmap.keys() {
        if !_leftmap.contains_key(name) {
            result.rightonly.push(name.clone());
        }
    }

    for (name, leftitem) in &_leftmap {
        if let Some(rightitem) = _rightmap.get(name) {
            let fielddiffs = compare_fields(name, leftitem, rightitem);
            result.diffs.extend(fielddiffs);
        }
    }

    result
}

#[allow(dead_code)]
fn compare_fields(name: &str, left: &Value, right: &Value) -> Vec<Difference> {
    let mut diffs = Vec::new();

    let leftreplicas = left["spec"]["replicas"].as_i64();
    let rightreplicas = right["spec"]["replicas"].as_i64();
    if leftreplicas != rightreplicas {
        diffs.push(Difference {
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
                    resource: name.to_string(),
                    path: format!("spec.template.spec.containers[{}].image", i),
                    kind: DiffType::Modified,
                    leftval: leftimg.map(String::from),
                    rightval: rightimg.map(String::from),
                });
            }
        }
    }

    if let (Some(_leftlabels), Some(_rightlabels)) = (
        left["metadata"]["labels"].as_object(),
        right["metadata"]["labels"].as_object(),
    ) {
        for (key, leftval) in _leftlabels {
            match _rightlabels.get(key) {
                Some(rightval) if rightval != leftval => {
                    diffs.push(Difference {
                        resource: name.to_string(),
                        path: format!("metadata.labels.{}", key),
                        kind: DiffType::Modified,
                        leftval: leftval.as_str().map(String::from),
                        rightval: rightval.as_str().map(String::from),
                    });
                }
                None => {
                    diffs.push(Difference {
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
                    resource: name.to_string(),
                    path: format!("metadata.labels.{}", key),
                    kind: DiffType::Added,
                    leftval: None,
                    rightval: _rightlabels[key].as_str().map(String::from),
                });
            }
        }
    }

    diffs
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_no_differences() {
        let data = json!({
            "items": [{
                "metadata": {"name": "test"},
                "spec": {"replicas": 3}
            }]
        });

        let result = compare_deployments(&data, &data);
        assert!(result.is_empty());
    }

    #[test]
    fn test_replica_difference() {
        let left = json!({
            "items": [{
                "metadata": {"name": "test"},
                "spec": {"replicas": 2}
            }]
        });

        let right = json!({
            "items": [{
                "metadata": {"name": "test"},
                "spec": {"replicas": 5}
            }]
        });

        let result = compare_deployments(&left, &right);
        assert_eq!(result.diffs.len(), 1);
        assert_eq!(result.diffs[0].path, "spec.replicas");
    }

    #[test]
    fn test_resource_only_in_left() {
        let left = json!({
            "items": [{
                "metadata": {"name": "test"},
                "spec": {"replicas": 2}
            }]
        });

        let right = json!({"items": []});

        let result = compare_deployments(&left, &right);
        assert_eq!(result.leftonly.len(), 1);
        assert_eq!(result.leftonly[0], "test");
    }
}
