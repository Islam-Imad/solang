// SPDX-License-Identifier: Apache-2.0

use crate::SorobanEnv;
use soroban_sdk::{
    contracttype, testutils::Address as _, testutils::Ledger, Address, FromVal, IntoVal, Val,
};

const MINT_LOCK_SRC: &str = r#"
contract mint_lock {
    struct MinterConfig {
        int128 limit;
        uint32 epoch_length;
    }

    struct MinterStats {
        int128 consumed_limit;
    }

    struct MinterInfo {
        MinterConfig config;
        uint32 epoch;
        MinterStats minter_stats;
    }

    address instance admin;

    mapping(address => mapping(address => MinterConfig)) minters;
    mapping(address => mapping(address => bool)) minter_exists;

    mapping(address =>
        mapping(address =>
            mapping(uint32 =>
                mapping(uint32 => MinterStats)))) stats;

    constructor(address admin_) {
        admin = admin_;
    }

    function set_admin(address new_admin) public {
        admin.requireAuth();
        admin = new_admin;
    }

    function get_admin() public view returns (address) {
        return admin;
    }

    function set_minter(
        address contract_,
        address minter_,
        MinterConfig memory config
    ) public {
        admin.requireAuth();
        minters[contract_][minter_] = config;
        minter_exists[contract_][minter_] = true;
    }

    function minter(address contract_, address minter_)
        public
        view
        returns (MinterInfo memory)
    {
        require(minter_exists[contract_][minter_], "not authorized minter");
        MinterConfig memory config = minters[contract_][minter_];
        uint32 epoch = uint32(block.number / uint64(config.epoch_length));
        MinterStats memory minter_stats = stats[contract_][minter_][config.epoch_length][epoch];
        return MinterInfo({config: config, epoch: epoch, minter_stats: minter_stats});
    }

    function mint(
        address contract_,
        address minter_,
        address to,
        int128 amount
    ) public {
        minter_.requireAuth();

        require(amount >= 0, "negative amount");

        // The admin can always mint; everyone else is rate-limited per epoch.
        if (admin != minter_) {
            require(minter_exists[contract_][minter_], "not authorized minter");
            MinterConfig memory config = minters[contract_][minter_];

            uint32 epoch = uint32(block.number / uint64(config.epoch_length));
            MinterStats memory minter_stats = stats[contract_][minter_][config.epoch_length][epoch];
            int128 new_consumed = minter_stats.consumed_limit + amount;
            require(new_consumed <= config.limit, "daily limit insufficient");
            minter_stats.consumed_limit = new_consumed;
            stats[contract_][minter_][config.epoch_length][epoch] = minter_stats;
        }

        // Dispatch the actual mint into the wrapped token contract.
        bytes memory payload = abi.encode("mint", to, amount);
        (bool success, bytes memory returndata) = contract_.call(payload);
    }
}
"#;

const MINT_TARGET_SRC: &str = r#"
contract mint_target {
    mapping(address => int128) balances;

    function mint(address to, int128 amount) public {
        balances[to] = balances[to] + amount;
    }

    function balance_of(address who) public view returns (int128) {
        return balances[who];
    }
}
"#;

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinterConfig {
    pub limit: i128,
    pub epoch_length: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinterStats {
    pub consumed_limit: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinterInfo {
    pub config: MinterConfig,
    pub epoch: u32,
    pub minter_stats: MinterStats,
}

const EPOCH_LENGTH: u32 = 10_000;
const LEDGER_EPOCH: u32 = 150;
const MID_EPOCH_SEQUENCE: u32 = EPOCH_LENGTH * LEDGER_EPOCH + EPOCH_LENGTH / 2;

fn deploy_lock(runtime: &mut SorobanEnv, admin: &Address) -> Address {
    runtime.deploy_contract_with_args(MINT_LOCK_SRC, (admin.clone(),))
}

fn set_minter(
    runtime: &SorobanEnv,
    lock: &Address,
    token: &Address,
    minter: &Address,
    config: &MinterConfig,
) {
    runtime.invoke_contract(
        lock,
        "set_minter",
        vec![
            token.clone().into_val(&runtime.env),
            minter.clone().into_val(&runtime.env),
            config.clone().into_val(&runtime.env),
        ],
    );
}

fn mint_args(
    runtime: &SorobanEnv,
    token: &Address,
    minter: &Address,
    to: &Address,
    amount: i128,
) -> Vec<Val> {
    vec![
        token.clone().into_val(&runtime.env),
        minter.clone().into_val(&runtime.env),
        to.clone().into_val(&runtime.env),
        amount.into_val(&runtime.env),
    ]
}

fn mint(
    runtime: &SorobanEnv,
    lock: &Address,
    token: &Address,
    minter: &Address,
    to: &Address,
    amount: i128,
) {
    runtime.invoke_contract(lock, "mint", mint_args(runtime, token, minter, to, amount));
}

fn try_mint(
    runtime: &SorobanEnv,
    lock: &Address,
    token: &Address,
    minter: &Address,
    to: &Address,
    amount: i128,
) -> Vec<String> {
    runtime.invoke_contract_expect_error(
        lock,
        "mint",
        mint_args(runtime, token, minter, to, amount),
    )
}

fn balance_of(runtime: &SorobanEnv, token: &Address, who: &Address) -> i128 {
    let val = runtime.invoke_contract(
        token,
        "balance_of",
        vec![who.clone().into_val(&runtime.env)],
    );
    i128::from_val(&runtime.env, &val)
}

fn minter_info(
    runtime: &SorobanEnv,
    lock: &Address,
    token: &Address,
    minter: &Address,
) -> MinterInfo {
    MinterInfo::from_val(
        &runtime.env,
        &runtime.invoke_contract(
            lock,
            "minter",
            vec![
                token.clone().into_val(&runtime.env),
                minter.clone().into_val(&runtime.env),
            ],
        ),
    )
}

#[test]
fn mint_lock_admin_accessor() {
    let mut runtime = SorobanEnv::new();
    let admin = Address::generate(&runtime.env);
    let addr = deploy_lock(&mut runtime, &admin);

    let got = Address::from_val(
        &runtime.env,
        &runtime.invoke_contract(&addr, "get_admin", vec![]),
    );
    assert_eq!(got, admin);
}

#[test]
fn mint_lock_set_admin_rotates() {
    let mut runtime = SorobanEnv::new();
    let admin = Address::generate(&runtime.env);
    let new_admin = Address::generate(&runtime.env);
    let addr = deploy_lock(&mut runtime, &admin);

    runtime.env.mock_all_auths();
    runtime.invoke_contract(
        &addr,
        "set_admin",
        vec![new_admin.clone().into_val(&runtime.env)],
    );

    let got = Address::from_val(
        &runtime.env,
        &runtime.invoke_contract(&addr, "get_admin", vec![]),
    );
    assert_eq!(got, new_admin);
}

#[test]
fn mint_lock_admin_can_always_mint() {
    let mut runtime = SorobanEnv::new();
    let admin = Address::generate(&runtime.env);
    let user = Address::generate(&runtime.env);

    let token = runtime.deploy_contract(MINT_TARGET_SRC);
    let lock = deploy_lock(&mut runtime, &admin);

    runtime.env.mock_all_auths();

    mint(&runtime, &lock, &token, &admin, &user, 123);
    assert_eq!(balance_of(&runtime, &token, &user), 123);
}

#[test]
fn mint_lock_authorized_minter_mints_and_tracks() {
    let mut runtime = SorobanEnv::new();
    let admin = Address::generate(&runtime.env);
    let minter = Address::generate(&runtime.env);
    let user = Address::generate(&runtime.env);

    let token = runtime.deploy_contract(MINT_TARGET_SRC);
    let lock = deploy_lock(&mut runtime, &admin);

    runtime.env.mock_all_auths();
    runtime.env.ledger().set_sequence_number(MID_EPOCH_SEQUENCE);

    let config = MinterConfig {
        limit: 100,
        epoch_length: EPOCH_LENGTH,
    };
    set_minter(&runtime, &lock, &token, &minter, &config);

    mint(&runtime, &lock, &token, &minter, &user, 97);
    assert_eq!(balance_of(&runtime, &token, &user), 97);

    let info = minter_info(&runtime, &lock, &token, &minter);
    assert_eq!(info.config, config);
    assert_eq!(info.epoch, LEDGER_EPOCH);
    assert_eq!(info.minter_stats.consumed_limit, 97);
}

#[test]
fn mint_lock_enforces_daily_limit() {
    let mut runtime = SorobanEnv::new();
    let admin = Address::generate(&runtime.env);
    let minter = Address::generate(&runtime.env);
    let user = Address::generate(&runtime.env);

    let token = runtime.deploy_contract(MINT_TARGET_SRC);
    let lock = deploy_lock(&mut runtime, &admin);

    runtime.env.mock_all_auths();
    runtime.env.ledger().set_sequence_number(MID_EPOCH_SEQUENCE);

    set_minter(
        &runtime,
        &lock,
        &token,
        &minter,
        &MinterConfig {
            limit: 100,
            epoch_length: EPOCH_LENGTH,
        },
    );

    // Spend 60, then 40, exactly exhausting the limit of 100.
    mint(&runtime, &lock, &token, &minter, &user, 60);
    mint(&runtime, &lock, &token, &minter, &user, 40);
    assert_eq!(balance_of(&runtime, &token, &user), 100);
    assert_eq!(
        minter_info(&runtime, &lock, &token, &minter)
            .minter_stats
            .consumed_limit,
        100
    );

    // One more unit would exceed the limit — rejected, and nothing changes.
    let logs = try_mint(&runtime, &lock, &token, &minter, &user, 1);
    assert!(logs.iter().any(|e| e.contains("require condition failed")));
    assert_eq!(balance_of(&runtime, &token, &user), 100);
    assert_eq!(
        minter_info(&runtime, &lock, &token, &minter)
            .minter_stats
            .consumed_limit,
        100
    );
}

#[test]
fn mint_lock_rejects_unauthorized_minter() {
    let mut runtime = SorobanEnv::new();
    let admin = Address::generate(&runtime.env);
    let stranger = Address::generate(&runtime.env);
    let user = Address::generate(&runtime.env);

    let token = runtime.deploy_contract(MINT_TARGET_SRC);
    let lock = deploy_lock(&mut runtime, &admin);

    runtime.env.mock_all_auths();

    let logs = try_mint(&runtime, &lock, &token, &stranger, &user, 1);
    assert!(logs.iter().any(|e| e.contains("require condition failed")));
    assert_eq!(balance_of(&runtime, &token, &user), 0);
}

#[test]
fn mint_lock_disallows_negative_amount_from_admin() {
    let mut runtime = SorobanEnv::new();
    let admin = Address::generate(&runtime.env);
    let user = Address::generate(&runtime.env);

    let token = runtime.deploy_contract(MINT_TARGET_SRC);
    let lock = deploy_lock(&mut runtime, &admin);

    runtime.env.mock_all_auths();

    let logs = try_mint(&runtime, &lock, &token, &admin, &user, -123);
    assert!(logs.iter().any(|e| e.contains("require condition failed")));
    assert_eq!(balance_of(&runtime, &token, &user), 0);
}

#[test]
fn mint_lock_disallows_negative_amount_from_minter() {
    let mut runtime = SorobanEnv::new();
    let admin = Address::generate(&runtime.env);
    let minter = Address::generate(&runtime.env);
    let user = Address::generate(&runtime.env);

    let token = runtime.deploy_contract(MINT_TARGET_SRC);
    let lock = deploy_lock(&mut runtime, &admin);

    runtime.env.mock_all_auths();
    runtime.env.ledger().set_sequence_number(MID_EPOCH_SEQUENCE);

    let config = MinterConfig {
        limit: 100,
        epoch_length: EPOCH_LENGTH,
    };
    set_minter(&runtime, &lock, &token, &minter, &config);

    let logs = try_mint(&runtime, &lock, &token, &minter, &user, -1000);
    assert!(logs.iter().any(|e| e.contains("require condition failed")));
    assert_eq!(balance_of(&runtime, &token, &user), 0);

    // The rejected mint left the epoch's stats at zero.
    let info = minter_info(&runtime, &lock, &token, &minter);
    assert_eq!(info.config, config);
    assert_eq!(info.epoch, LEDGER_EPOCH);
    assert_eq!(info.minter_stats.consumed_limit, 0);
}

#[test]
fn mint_lock_set_and_read_minter() {
    let mut runtime = SorobanEnv::new();
    let admin = Address::generate(&runtime.env);
    let token = Address::generate(&runtime.env);
    let minter = Address::generate(&runtime.env);
    let lock = deploy_lock(&mut runtime, &admin);

    runtime.env.mock_all_auths();

    let config = MinterConfig {
        limit: 1000,
        epoch_length: 100,
    };
    set_minter(&runtime, &lock, &token, &minter, &config);

    let info = minter_info(&runtime, &lock, &token, &minter);
    assert_eq!(info.config, config);
    assert_eq!(info.minter_stats.consumed_limit, 0);
}
