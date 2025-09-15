use {
    alloy_primitives::{U256, utils::parse_units},
    serde::{Deserialize, Deserializer, Serializer, de::Error as DeError},
};

fn alloy_to_decimal_str(v: &U256) -> String {
    // serialize as decimal wei by default (unambiguous)
    v.to_string()
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    // Accept either a string (with optional unit) or a bare integer
    // We deserialize into a serde_json::Value-like enum via an untagged helper.
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Str(String),
        U64(u64),
        U128(u128),
        // If you expect very large bare numbers in TOML/YAML, add String only and
        // let users quote them. Most formats cap bare integer size.
    }

    let raw = Raw::deserialize(deserializer)?;

    let (num_str, unit) = match raw {
        Raw::U64(n) => (n.to_string(), "wei".to_string()),
        Raw::U128(n) => (n.to_string(), "wei".to_string()),
        Raw::Str(s0) => {
            let s = s0.trim();
            if s.is_empty() {
                return Err(D::Error::custom("empty value"));
            }

            // Allow "1 ether", "1ether", "2 gwei", "100 wei", or just a big decimal string.
            // Split numeric and alpha parts from the right.
            let mut letters = String::new();

            // If there's whitespace, just split on it first; else fall back to scanning suffix.
            if let Some((lhs, rhs)) = s.rsplit_once(char::is_whitespace) {
                let lhs = lhs.trim();
                let rhs = rhs.trim();
                if rhs.eq_ignore_ascii_case("ether")
                    || rhs.eq_ignore_ascii_case("eth")
                    || rhs.eq_ignore_ascii_case("gwei")
                    || rhs.eq_ignore_ascii_case("wei")
                {
                    (lhs.to_string(), rhs.to_string())
                } else {
                    // treat entire string as number (in wei)
                    (s.to_string(), "wei".to_string())
                }
            } else {
                // No whitespace: split trailing letters
                for ch in s.chars().rev() {
                    if ch.is_ascii_alphabetic() {
                        letters.push(ch);
                    } else {
                        break;
                    }
                }
                if !letters.is_empty() {
                    let unit_rev: String = letters.chars().collect();
                    let unit_clean = unit_rev.chars().rev().collect::<String>();
                    let num_len = s.len() - unit_clean.len();
                    if num_len == 0 {
                        return Err(D::Error::custom("missing number before unit"));
                    }
                    let number_part = &s[..num_len];
                    (number_part.to_string(), unit_clean)
                } else {
                    (s.to_string(), "wei".to_string())
                }
            }
        }
    };

    // Normalize unit
    let unit_norm = match unit.to_ascii_lowercase().as_str() {
        "eth" => "ether",
        "ether" => "ether",
        "gwei" => "gwei",
        "wei" => "wei",
        other => return Err(D::Error::custom(format!("unknown unit: {other}"))),
    };

    // Strip underscores from the numeric part to allow 1_000_000 style
    let num_clean = num_str.replace('_', "");

    // parse_units handles decimals for ether/gwei; for wei, it expects integer
    let parsed = parse_units(&num_clean, unit_norm)
        .map_err(|e| D::Error::custom(format!("invalid value `{num_clean} {unit_norm}`: {e}")))?
        .into();
    Ok(parsed)
}

pub fn serialize<S>(v: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Serialize back as decimal wei string to avoid surprises
    serializer.serialize_str(&alloy_to_decimal_str(v))
}
