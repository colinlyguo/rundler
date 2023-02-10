use ethers::contract::abigen;
use ethers::types::{Address, Bytes, U256};
use serde::{Deserialize, Deserializer};

abigen!(EntryPoint, "contracts/out/EntryPoint.sol/EntryPoint.json");

// abigen has generated a UserOperation struct, but we need to make this struct
// JSON-deserializable so we can receive it through JSON-RPC requests. Do so
// by defining a UserOperationDef helper struct, which is how one adds
// deserialization to structs from a remote crate in serde_json, even though
// UserOperation isn't actually remote.

impl<'de> Deserialize<'de> for UserOperation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        UserOperationDef::deserialize(deserializer)
    }
}

#[derive(Default, Deserialize)]
#[serde(default, remote = "UserOperation", rename_all = "camelCase")]
struct UserOperationDef {
    // Fields copied from the code generated by abigen
    sender: Address,
    nonce: U256,
    init_code: Bytes,
    call_data: Bytes,
    call_gas_limit: U256,
    verification_gas_limit: U256,
    pre_verification_gas: U256,
    max_fee_per_gas: U256,
    max_priority_fee_per_gas: U256,
    paymaster_and_data: Bytes,
    signature: Bytes,
}

#[cfg(test)]
mod test {
    use super::*;
    use ethers::utils::hex;

    #[test]
    fn test_json_deserialization() {
        let json = r#"
{
  "sender": "0xBec18eFCA4eEf2E4BA54E5a07534604A4Dd3191d",
  "nonce": "0x5",
  "callData": "0x0123456701234567012345670123456701234567012345670123456701234567"
}"#;
        let deserialized: UserOperation =
            serde_json::from_str(json).expect("json should deserialize");
        let expected = UserOperation {
            sender: Address::from_slice(
                &hex::decode("Bec18eFCA4eEf2E4BA54E5a07534604A4Dd3191d").unwrap(),
            ),
            nonce: U256::from(5),
            call_data: Bytes::from(
                hex::decode("0123456701234567012345670123456701234567012345670123456701234567")
                    .unwrap(),
            ),
            ..UserOperation::default()
        };
        assert_eq!(deserialized, expected)
    }
}
