use {
    alloy_primitives::{
        U256, uint,
        utils::{format_units_with, parse_units},
    },
    serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError},
};

#[test]
fn test_parse_units() {
    let res = parse_units("0.001", "wei");
    println!("{:?}", res);

    let res = format_units_with(1, "gwei", Default::default());

    println!("{:?}", res);
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Str(String),
        U64(u64),
        U128(u128),
    }

    let raw = Raw::deserialize(deserializer)?;

    let (num_str, unit_str) = match raw {
        Raw::U64(n) => (n.to_string(), String::from("wei")),
        Raw::U128(n) => (n.to_string(), String::from("wei")),
        Raw::Str(s0) => {
            let s = s0.trim();
            if s.is_empty() {
                return Err(D::Error::custom("empty value"));
            }

            let mut unit_len = 0usize;
            for ch in s.chars().rev() {
                if ch.is_ascii_alphabetic() {
                    unit_len += ch.len_utf8();
                } else if ch.is_whitespace() && unit_len == 0 {
                    continue;
                } else {
                    break;
                }
            }

            if unit_len > 0 {
                let split_at = s.len() - unit_len;
                let (num_part, unit_part) = s.split_at(split_at);
                (num_part.trim().to_string(), unit_part.trim().to_string())
            } else {
                (s.to_string(), String::from("wei"))
            }
        }
    };

    let unit_norm = match unit_str.to_ascii_lowercase().as_str() {
        "eth" | "ether" => "ether",
        "gwei" => "gwei",
        "wei" => "wei",
        other => return Err(D::Error::custom(format!("unknown unit: {other}"))),
    };

    let mut cleaned = String::with_capacity(num_str.len());
    let mut seen_dot = false;
    let mut prev: Option<char> = None;

    for (i, ch) in num_str.chars().enumerate() {
        match ch {
            '0'..='9' => {
                cleaned.push(ch);
            }
            '.' => {
                if unit_norm == "wei" {
                    return Err(D::Error::custom("decimals not allowed for wei"));
                }
                if seen_dot {
                    return Err(D::Error::custom("multiple decimal points"));
                }
                if matches!(prev, Some('_')) {
                    return Err(D::Error::custom(
                        "underscore cannot be adjacent to decimal point",
                    ));
                }
                seen_dot = true;
                cleaned.push('.');
            }
            '_' => {
                if i == 0 || i == num_str.len() - 1 {
                    return Err(D::Error::custom("underscore cannot be at start or end"));
                }
                if matches!(prev, Some('_')) {
                    return Err(D::Error::custom("consecutive underscores are not allowed"));
                }
                if matches!(prev, Some('.')) {
                    return Err(D::Error::custom(
                        "underscore cannot be adjacent to decimal point",
                    ));
                }
                cleaned.push('_');
            }
            c if c.is_whitespace() => {
                return Err(D::Error::custom("whitespace inside number is not allowed"));
            }
            _ => {
                return Err(D::Error::custom(format!(
                    "invalid character in number: `{ch}`"
                )));
            }
        }
        prev = Some(ch);
    }

    let cleaned = cleaned.replace('_', "");
    if cleaned.is_empty() || cleaned == "." {
        return Err(D::Error::custom("invalid empty/decimal-only number"));
    }
    if unit_norm == "wei" && cleaned.contains('.') {
        return Err(D::Error::custom("decimals not allowed for wei"));
    }
    let parsed = parse_units(&cleaned, unit_norm)
        .map_err(|e| D::Error::custom(format!("invalid value `{cleaned} {unit_norm}`: {e}")))?
        .into();

    Ok(parsed)
}

#[inline]
fn pick_unit_human_friendly(v: &U256) -> &'static str {
    // 0.0001 ether = 1e14 wei
    const ETHER_MIN: U256 = uint!(100_000_000_000_000U256);
    // 0.0001 gwei = 1e5 wei
    const GWEI_MIN: U256 = uint!(100_000U256);

    if *v >= ETHER_MIN {
        "ether"
    } else if *v >= GWEI_MIN {
        "gwei"
    } else {
        "wei"
    }
}

pub fn serialize<S>(v: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if serializer.is_human_readable() {
        use serde::ser::Error;

        let unit = pick_unit_human_friendly(v);
        let s = format_units_with(*v, unit, Default::default()).map_err(S::Error::custom)?;

        serializer.serialize_str(&format!("{s} {unit}"))
    } else {
        v.serialize(serializer)
    }
}
