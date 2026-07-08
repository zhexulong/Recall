use serde_json::{Value, json};

pub fn manifest_json() -> Value {
    json!({
        "name": "reflect",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": 1,
        "min_recall": "0.2.10"
    })
}

#[cfg(test)]
mod tests {
    use super::manifest_json;

    #[test]
    fn manifest_matches_extension_contract() {
        let manifest = manifest_json();
        assert_eq!(manifest["name"], "reflect");
        assert_eq!(manifest["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(manifest["protocol"], 1);
        assert_eq!(manifest["min_recall"], "0.2.10");
    }
}
