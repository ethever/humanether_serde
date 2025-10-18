use {
    alloy_primitives::{U256, uint},
    humanether_serde,
};

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
struct A {
    #[serde(with = "humanether_serde")]
    gas: U256,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
struct B {
    #[serde(with = "humanether_serde")]
    gas: u128,
}

fn main() {
    let a = A {
        gas: uint!(1000000000000000_U256),
    };

    println!("{}", serde_json::to_string(&a).unwrap());

    // Example JSON string (currently unused)
    let s = r#"{"gas":"100 gwei"}"#;
    println!("Example JSON string: {}", s);

    let res = serde_json::from_str::<A>(s);
    println!("{:?}", res);
}
