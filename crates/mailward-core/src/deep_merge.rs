//! Deep-merge for per-account config/rules overrides.

use serde_yaml_ng::Value;

/// Deep-merges `over` onto `base`, returning a new value:
/// nested mappings merge recursively, while sequences and scalars in `over`
/// REPLACE the corresponding base value. Mirrors the v1 `deepMerge` used to
/// apply per-account overrides over the shared YAML config.
pub fn deep_merge(base: Value, over: Value) -> Value {
    match (base, over) {
        (Value::Mapping(mut base), Value::Mapping(over)) => {
            for (key, value) in over {
                let merged = match base.remove(&key) {
                    Some(existing) => deep_merge(existing, value),
                    None => value,
                };
                base.insert(key, merged);
            }
            Value::Mapping(base)
        }
        // Sequences and scalars replace; a mismatched shape replaces too.
        (_, over) => over,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml(input: &str) -> Value {
        serde_yaml_ng::from_str(input).unwrap()
    }

    #[test]
    fn objects_merge_arrays_and_scalars_replace() {
        let base = yaml("a:\n  x: 1\n  y: [1, 2]\nb: keep\n");
        let over = yaml("a:\n  y: [9]\n  z: 3\n");
        let merged = deep_merge(base, over);
        assert_eq!(merged, yaml("a:\n  x: 1\n  y: [9]\n  z: 3\nb: keep\n"));
    }
}
