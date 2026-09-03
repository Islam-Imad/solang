// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, SorobanEnv};
use soroban_sdk::{testutils::Address as _, Address, FromVal, IntoVal};

const SETTER_SRC: &str = r#"contract addr_cmp {
    address instance admin;

    function set_admin(address a) public {
        admin = a;
    }

    function is_admin(address who) public view returns (bool) {
        return admin == who;
    }

    function not_admin(address who) public view returns (bool) {
        return admin != who;
    }

    function same(address a, address b) public pure returns (bool) {
        return a == b;
    }
}"#;

#[test]
fn address_equal_storage_vs_arg() {
    let runtime = build_solidity(SETTER_SRC, |_| {});
    let admin = Address::generate(&runtime.env);
    let other = Address::generate(&runtime.env);
    let addr = runtime.contracts.first().unwrap().clone();

    runtime.invoke_contract(
        &addr,
        "set_admin",
        vec![admin.clone().into_val(&runtime.env)],
    );

    let res = runtime.invoke_contract(
        &addr,
        "is_admin",
        vec![admin.clone().into_val(&runtime.env)],
    );
    assert!(bool::from_val(&runtime.env, &res));

    let res = runtime.invoke_contract(
        &addr,
        "is_admin",
        vec![other.clone().into_val(&runtime.env)],
    );
    assert!(!bool::from_val(&runtime.env, &res));
}

#[test]
fn address_not_equal_storage_vs_arg() {
    let runtime = build_solidity(SETTER_SRC, |_| {});
    let admin = Address::generate(&runtime.env);
    let other = Address::generate(&runtime.env);
    let addr = runtime.contracts.first().unwrap().clone();

    runtime.invoke_contract(
        &addr,
        "set_admin",
        vec![admin.clone().into_val(&runtime.env)],
    );

    let res = runtime.invoke_contract(
        &addr,
        "not_admin",
        vec![admin.clone().into_val(&runtime.env)],
    );
    assert!(!bool::from_val(&runtime.env, &res));

    let res = runtime.invoke_contract(
        &addr,
        "not_admin",
        vec![other.clone().into_val(&runtime.env)],
    );
    assert!(bool::from_val(&runtime.env, &res));
}

#[test]
fn address_equal_arg_vs_arg() {
    let runtime = build_solidity(SETTER_SRC, |_| {});
    let a = Address::generate(&runtime.env);
    let b = Address::generate(&runtime.env);
    let addr = runtime.contracts.first().unwrap().clone();

    let res = runtime.invoke_contract(
        &addr,
        "same",
        vec![
            a.clone().into_val(&runtime.env),
            a.clone().into_val(&runtime.env),
        ],
    );
    assert!(bool::from_val(&runtime.env, &res));

    let res = runtime.invoke_contract(
        &addr,
        "same",
        vec![
            a.clone().into_val(&runtime.env),
            b.clone().into_val(&runtime.env),
        ],
    );
    assert!(!bool::from_val(&runtime.env, &res));
}

const CONSTRUCTOR_SRC: &str = r#"contract addr_cmp_ctor {
    address instance admin;

    constructor(address admin_) {
        admin = admin_;
    }

    function is_admin(address who) public view returns (bool) {
        return admin == who;
    }
}"#;

#[test]
fn address_equal_constructor_stored() {
    let mut runtime = SorobanEnv::new();
    let admin = Address::generate(&runtime.env);
    let other = Address::generate(&runtime.env);

    let addr = runtime.deploy_contract_with_args(CONSTRUCTOR_SRC, (admin.clone(),));

    let res = runtime.invoke_contract(
        &addr,
        "is_admin",
        vec![admin.clone().into_val(&runtime.env)],
    );
    assert!(bool::from_val(&runtime.env, &res));

    let res = runtime.invoke_contract(
        &addr,
        "is_admin",
        vec![other.clone().into_val(&runtime.env)],
    );
    assert!(!bool::from_val(&runtime.env, &res));
}
