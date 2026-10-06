use std::path::PathBuf;

fn contracts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("contracts")
}

#[test]
fn schema_files_are_json() {
    for name in ["capabilities.json", "tools.json", "ipc.json", "transaction.json"] {
        let path = contracts_dir().join(name);
        let raw = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing {name}"));
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert!(v.is_object(), "{name} must be an object");
    }
}
