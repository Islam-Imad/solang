// SPDX-License-Identifier: Apache-2.0

use crate::{build_wasm, SorobanEnv};
use soroban_sdk::{
    testutils::Address as _, Address, Bytes, BytesN, FromVal, IntoVal, String, Vec as SVec,
};

const DEPLOYER_SRC: &str = r#"
contract deployer {
    address instance admin;

    constructor(address admin_) {
        admin = admin_;
    }

    function deploy(bytes32 wasm_hash, bytes32 salt, uint32 init_value)
        public
        returns (address)
    {
        admin.requireAuth();
        return deployContract(wasm_hash, salt, init_value);
    }
}
"#;

const DEPLOYED_SRC: &str = r#"
contract deployed {
    uint32 instance stored_value;

    constructor(uint32 value) {
        stored_value = value;
    }

    function value() public view returns (uint32) {
        return stored_value;
    }
}
"#;

const DEPLOYER_MULTI_SRC: &str = r#"
contract deployer_multi {
    function deploy(
        bytes32 wasm_hash,
        bytes32 salt,
        int128 n,
        string memory s,
        uint32[] memory arr
    ) public returns (address) {
        return deployContract(wasm_hash, salt, n, s, arr);
    }
}
"#;

const DEPLOYED_MULTI_SRC: &str = r#"
contract deployed_multi {
    int128 n;
    string s;
    uint32[] arr;

    constructor(int128 n_, string memory s_, uint32[] memory arr_) {
        n = n_;
        s = s_;
        arr = arr_;
    }

    function get_n() public view returns (int128) {
        return n;
    }

    function get_s() public view returns (string memory) {
        return s;
    }

    function get_arr() public view returns (uint32[] memory) {
        return arr;
    }
}
"#;

// A factory that forwards a string *literal* straight to `deployContract`,
// exercising the `resolve_encode_arg` literal handling (a `BytesLiteral` is
// cast to `string` before encoding) shared with `to_xdr`/`abi.encode`.
const DEPLOYER_LITERAL_SRC: &str = r#"
contract deployer_literal {
    function deploy(bytes32 wasm_hash, bytes32 salt) public returns (address) {
        return deployContract(wasm_hash, salt, "solang-literal");
    }
}
"#;

// The child of the string-literal factory: stores its constructor string.
const DEPLOYED_STRING_SRC: &str = r#"
contract deployed_string {
    string s;

    constructor(string memory s_) {
        s = s_;
    }

    function get_s() public view returns (string memory) {
        return s;
    }
}
"#;

fn upload_child(runtime: &SorobanEnv) -> BytesN<32> {
    let child_wasm = build_wasm(DEPLOYED_SRC).0;
    let child_bytes = Bytes::from_slice(&runtime.env, &child_wasm);
    runtime.env.deployer().upload_contract_wasm(child_bytes)
}

fn salt(runtime: &SorobanEnv, byte: u8) -> BytesN<32> {
    BytesN::from_array(&runtime.env, &[byte; 32])
}

fn deploy(
    runtime: &SorobanEnv,
    factory: &Address,
    wasm_hash: &BytesN<32>,
    salt: &BytesN<32>,
    init_value: u32,
) -> Address {
    let deployed = runtime.invoke_contract(
        factory,
        "deploy",
        vec![
            wasm_hash.clone().into_val(&runtime.env),
            salt.clone().into_val(&runtime.env),
            init_value.into_val(&runtime.env),
        ],
    );
    Address::from_val(&runtime.env, &deployed)
}

fn read_value(runtime: &SorobanEnv, contract: &Address) -> u32 {
    let val = runtime.invoke_contract(contract, "value", vec![]);
    u32::from_val(&runtime.env, &val)
}

#[test]
fn example_deployer_deploys_and_initializes() {
    let mut runtime = SorobanEnv::new();
    runtime.env.mock_all_auths();

    let admin = Address::generate(&runtime.env);
    let factory = runtime.deploy_contract_with_args(DEPLOYER_SRC, (admin.clone(),));

    let wasm_hash = upload_child(&runtime);
    let deployed = deploy(&runtime, &factory, &wasm_hash, &salt(&runtime, 0), 5);

    assert_eq!(read_value(&runtime, &deployed), 5);
}

#[test]
fn example_deployer_salt_determines_address() {
    let mut runtime = SorobanEnv::new();
    runtime.env.mock_all_auths();

    let admin = Address::generate(&runtime.env);
    let factory = runtime.deploy_contract_with_args(DEPLOYER_SRC, (admin.clone(),));
    let wasm_hash = upload_child(&runtime);

    let first = deploy(&runtime, &factory, &wasm_hash, &salt(&runtime, 1), 5);
    let second = deploy(&runtime, &factory, &wasm_hash, &salt(&runtime, 2), 7);

    assert_ne!(first, second);
    assert_eq!(read_value(&runtime, &first), 5);
    assert_eq!(read_value(&runtime, &second), 7);
}

#[test]
fn example_deployer_reuse_salt_traps() {
    let mut runtime = SorobanEnv::new();
    runtime.env.mock_all_auths();

    let admin = Address::generate(&runtime.env);
    let factory = runtime.deploy_contract_with_args(DEPLOYER_SRC, (admin.clone(),));
    let wasm_hash = upload_child(&runtime);
    let salt = salt(&runtime, 3);

    let first = deploy(&runtime, &factory, &wasm_hash, &salt, 5);
    assert_eq!(read_value(&runtime, &first), 5);

    let logs = runtime.invoke_contract_expect_error(
        &factory,
        "deploy",
        vec![
            wasm_hash.clone().into_val(&runtime.env),
            salt.into_val(&runtime.env),
            9u32.into_val(&runtime.env),
        ],
    );
    assert!(!logs.is_empty());
}

#[test]
fn example_deployer_multi_arg_constructor() {
    let mut runtime = SorobanEnv::new();
    runtime.env.mock_all_auths();

    let factory = runtime.deploy_contract(DEPLOYER_MULTI_SRC);

    let child_wasm = build_wasm(DEPLOYED_MULTI_SRC).0;
    let child_bytes = Bytes::from_slice(&runtime.env, &child_wasm);
    let wasm_hash = runtime.env.deployer().upload_contract_wasm(child_bytes);

    let s = String::from_str(&runtime.env, "solang");
    let arr: SVec<u32> = soroban_sdk::vec![&runtime.env, 10_u32, 20, 30];

    let deployed = runtime.invoke_contract(
        &factory,
        "deploy",
        vec![
            wasm_hash.into_val(&runtime.env),
            salt(&runtime, 4).into_val(&runtime.env),
            42_i128.into_val(&runtime.env),
            s.clone().into_val(&runtime.env),
            arr.clone().into_val(&runtime.env),
        ],
    );
    let deployed = Address::from_val(&runtime.env, &deployed);

    let n = runtime.invoke_contract(&deployed, "get_n", vec![]);
    assert_eq!(i128::from_val(&runtime.env, &n), 42);

    let got_s = runtime.invoke_contract(&deployed, "get_s", vec![]);
    assert_eq!(String::from_val(&runtime.env, &got_s), s);

    let got_arr = runtime.invoke_contract(&deployed, "get_arr", vec![]);
    assert_eq!(SVec::<u32>::from_val(&runtime.env, &got_arr), arr);
}

#[test]
fn example_deployer_string_literal_arg() {
    let mut runtime = SorobanEnv::new();
    runtime.env.mock_all_auths();

    let factory = runtime.deploy_contract(DEPLOYER_LITERAL_SRC);

    let child_wasm = build_wasm(DEPLOYED_STRING_SRC).0;
    let child_bytes = Bytes::from_slice(&runtime.env, &child_wasm);
    let wasm_hash = runtime.env.deployer().upload_contract_wasm(child_bytes);

    let deployed = runtime.invoke_contract(
        &factory,
        "deploy",
        vec![
            wasm_hash.into_val(&runtime.env),
            salt(&runtime, 5).into_val(&runtime.env),
        ],
    );
    let deployed = Address::from_val(&runtime.env, &deployed);

    // The literal was encoded as a string, so the child stored it verbatim.
    let got_s = runtime.invoke_contract(&deployed, "get_s", vec![]);
    assert_eq!(
        String::from_val(&runtime.env, &got_s),
        String::from_str(&runtime.env, "solang-literal")
    );
}
