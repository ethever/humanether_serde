use {
    alloy_primitives::{
        U256, uint,
        utils::{format_units_with, parse_units},
    },
    serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError},
};

pub trait FromU256: Sized {
    fn try_from_u256(v: U256) -> Result<Self, String>;
}

impl FromU256 for U256 {
    #[inline]
    fn try_from_u256(v: U256) -> Result<Self, String> {
        Ok(v)
    }
}

impl FromU256 for u128 {
    #[inline]
    fn try_from_u256(v: U256) -> Result<Self, String> {
        v.try_into()
            .map_err(|_| format!("value {v} does not fit into u128"))
    }
}

pub trait IntoU256 {
    fn into_u256(self) -> Result<U256, String>;
}

impl IntoU256 for U256 {
    #[inline]
    fn into_u256(self) -> Result<U256, String> {
        Ok(self)
    }
}

impl IntoU256 for u128 {
    #[inline]
    fn into_u256(self) -> Result<U256, String> {
        Ok(U256::from(self))
    }
}

#[test]
fn test_parse_units() {
    let res = parse_units("0.001", "wei");
    println!("{:?}", res);

    let res = format_units_with(1, "gwei", Default::default());

    println!("{:?}", res);
}

pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromU256,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Str(String),
        U64(u64),
        U128(u128),
    }

    let raw = Raw::deserialize(deserializer)?;

    // normalize to U256 first
    let u256 = match raw {
        Raw::U64(n) => U256::from(n),
        Raw::U128(n) => U256::from(n),
        Raw::Str(s) => parse_strict_number_with_unit::<D>(&s)?,
    };

    // then convert to T
    T::try_from_u256(u256).map_err(D::Error::custom)
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

// ---- STRICT parser shared by all T ----
fn parse_strict_number_with_unit<'de, D: Deserializer<'de>>(s0: &str) -> Result<U256, D::Error> {
    let s = s0.trim();
    if s.is_empty() {
        return Err(D::Error::custom("empty value"));
    }

    // split trailing ascii letters as unit (optionally preceded by spaces)
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

    let (num_str, unit_str) = if unit_len > 0 {
        let split_at = s.len() - unit_len;
        let (num_part, unit_part) = s.split_at(split_at);
        (num_part.trim().to_string(), unit_part.trim().to_string())
    } else {
        (s.to_string(), String::from("wei"))
    };

    let unit_norm = match unit_str.to_ascii_lowercase().as_str() {
        "eth" | "ether" => "ether",
        "gwei" => "gwei",
        "wei" => "wei",
        other => return Err(D::Error::custom(format!("unknown unit: {other}"))),
    };

    // strict character checks
    let mut cleaned = String::with_capacity(num_str.len());
    let mut seen_dot = false;
    let mut prev: Option<char> = None;

    for (i, ch) in num_str.chars().enumerate() {
        match ch {
            '0'..='9' => cleaned.push(ch),
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

    let parsed: U256 = parse_units(&cleaned, unit_norm)
        .map_err(|e| D::Error::custom(format!("invalid value `{cleaned} {unit_norm}`: {e}")))?
        .into();

    Ok(parsed)
}

pub fn serialize<S, T>(v: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Copy + IntoU256 + Serialize,
{
    if serializer.is_human_readable() {
        use serde::ser::Error;

        let v = v.into_u256().map_err(S::Error::custom)?;

        let unit = pick_unit_human_friendly(&v);
        let s = format_units_with(v, unit, Default::default()).map_err(S::Error::custom)?;

        serializer.serialize_str(&format!("{s} {unit}"))
    } else {
        v.serialize(serializer)
    }
}
