// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use soroban_sdk::{
    contracttype, testutils::Address as _, Address, Bytes, FromVal, IntoVal, String,
};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S1 {
    pub zulu: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S2 {
    pub zulu: u64,
    pub alpha: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S3 {
    pub zulu: u64,
    pub alpha: bool,
    pub mike: i32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S4 {
    pub zulu: u64,
    pub alpha: bool,
    pub mike: i32,
    pub bravo: Address,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S5 {
    pub zulu: u64,
    pub alpha: bool,
    pub mike: i32,
    pub bravo: Address,
    pub yankee: Bytes,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S6 {
    pub zulu: u64,
    pub alpha: bool,
    pub mike: i32,
    pub bravo: Address,
    pub yankee: Bytes,
    pub charlie: String,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct S7 {
    pub zulu: u64,
    pub alpha: bool,
    pub mike: i32,
    pub bravo: Address,
    pub yankee: Bytes,
    pub charlie: String,
    pub november: Bytes,
}

const FNS: &str = r#"
            function echo(S memory s) public pure returns (S memory) { return s; }
            function via_local(S memory s) public pure returns (S memory) {
                S memory t = s;
                return t;
            }
"#;

#[test]
fn struct_1_member() {
    let runtime = build_solidity(
        &format!(r#"contract test {{ struct S {{ uint64 zulu; }} {FNS} }}"#,),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S1 {
        zulu: 0x0123_4567_89AB_CDEF,
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S1::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S1::from_val(env, &res), input);
}

#[test]
fn struct_2_member() {
    let runtime = build_solidity(
        &format!(r#"contract test {{ struct S {{ uint64 zulu; bool alpha; }} {FNS} }}"#,),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S2 {
        zulu: 0x0123_4567_89AB_CDEF,
        alpha: true,
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S2::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S2::from_val(env, &res), input);
}

#[test]
fn struct_3_member() {
    let runtime = build_solidity(
        &format!(
            r#"contract test {{ struct S {{ uint64 zulu; bool alpha; int32 mike; }} {FNS} }}"#,
        ),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S3 {
        zulu: 0x0123_4567_89AB_CDEF,
        alpha: true,
        mike: -12345,
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S3::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S3::from_val(env, &res), input);
}

#[test]
fn struct_4_member() {
    let runtime = build_solidity(
        &format!(
            r#"contract test {{ struct S {{ uint64 zulu; bool alpha; int32 mike; address bravo; }} {FNS} }}"#,
        ),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S4 {
        zulu: 0x0123_4567_89AB_CDEF,
        alpha: true,
        mike: -12345,
        bravo: Address::generate(env),
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S4::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S4::from_val(env, &res), input);
}

#[test]
fn struct_5_member() {
    let runtime = build_solidity(
        &format!(
            r#"contract test {{ struct S {{ uint64 zulu; bool alpha; int32 mike; address bravo; bytes4 yankee; }} {FNS} }}"#,
        ),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S5 {
        zulu: 0x0123_4567_89AB_CDEF,
        alpha: true,
        mike: -12345,
        bravo: Address::generate(env),
        yankee: Bytes::from_array(env, &[0xDE, 0xAD, 0xBE, 0xEF]),
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S5::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S5::from_val(env, &res), input);
}

#[test]
fn struct_6_member() {
    let runtime = build_solidity(
        &format!(
            r#"contract test {{ struct S {{ uint64 zulu; bool alpha; int32 mike; address bravo; bytes4 yankee; string charlie; }} {FNS} }}"#,
        ),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S6 {
        zulu: 0x0123_4567_89AB_CDEF,
        alpha: true,
        mike: -12345,
        bravo: Address::generate(env),
        yankee: Bytes::from_array(env, &[0xDE, 0xAD, 0xBE, 0xEF]),
        charlie: String::from_str(env, "Solang!"),
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S6::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S6::from_val(env, &res), input);
}

#[test]
fn struct_7_member() {
    let runtime = build_solidity(
        &format!(
            r#"contract test {{ struct S {{ uint64 zulu; bool alpha; int32 mike; address bravo; bytes4 yankee; string charlie; bytes november; }} {FNS} }}"#,
        ),
        |_| {},
    );
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let input = S7 {
        zulu: 0x0123_4567_89AB_CDEF,
        alpha: true,
        mike: -12345,
        bravo: Address::generate(env),
        yankee: Bytes::from_array(env, &[0xDE, 0xAD, 0xBE, 0xEF]),
        charlie: String::from_str(env, "Solang!"),
        november: Bytes::from_array(env, &[0x01, 0x02, 0x03]),
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(env)]);
    assert_eq!(S7::from_val(env, &res), input);
    let res = runtime.invoke_contract(addr, "via_local", vec![input.clone().into_val(env)]);
    assert_eq!(S7::from_val(env, &res), input);
}
