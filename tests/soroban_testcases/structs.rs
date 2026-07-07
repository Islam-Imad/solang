// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use indexmap::Equivalent;
use soroban_sdk::{
    contracttype, testutils::Address as _, Address, Bytes, FromVal, IntoVal, Val, U256,
};

// SDK-side mirrors used as the ABI oracle: `#[contracttype]` encodes a named struct as an
// SCV_MAP keyed by field-name Symbol (fields sorted by name), which is exactly what our ABI
// encoder must produce. The Solidity structs below declare their fields in a NON-lexicographic
// order so a position-based encoder bug would surface as a field swap via `from_val`.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rec {
    pub zebra: u64,
    pub apple: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct IntRec {
    pub a: i32,
    pub b: i64,
    pub c: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ValRec {
    pub a: bool,
    pub b: u32,
    pub c: Bytes,
}

// Step-5 decode oracles. `Point`/`Line` guard the map-of-maps (nested) decode; `Wide`/`Big256`
// guard the Step-3 decode_i128/decode_i256 fix on struct fields (both the object range and a
// negative value). The Solidity structs mirror these names but declare fields in a scrambled
// order to prove decode is keyed by field name, not position.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: u64,
    pub y: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub a: Point,
    pub b: Point,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Wide {
    pub big: u128,
    pub neg: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Big256 {
    pub v: U256,
}

#[test]
fn get_fields_via_dot() {
    let runtime = build_solidity(
        r#"
        contract locker {
            struct Lock {
                uint64 release_time;
                address beneficiary;
                uint64 amount;
            }

            mapping(address => Lock) locks;

            function create_lock(
                uint64 release_time,
                address beneficiary,
                uint64 amount
            ) public returns (uint64) {
                Lock memory l = Lock({
                    release_time: release_time,
                    beneficiary: beneficiary,
                    amount: amount
                });

                locks[beneficiary] = l;
                return l.amount;
            }

            function get_lock_amount(address beneficiary) public view returns (uint64) {
                return locks[beneficiary].amount;
            }

            function get_lock_release(address beneficiary) public view returns (uint64) {
                return locks[beneficiary].release_time;
            }

            function get_lock_beneficiary(address key) public view returns (address) {
                return locks[key].beneficiary;
            }

            // Extended functionality: increase amount in-place and return new total
            function increase_lock_amount(address beneficiary, uint64 delta) public returns (uint64) {
                locks[beneficiary].amount += delta;
                return locks[beneficiary].amount;
            }

            // Extended functionality: move a lock to a different beneficiary
            function move_lock(address from, address to) public {
                Lock memory l = locks[from];
                require(l.amount != 0, "no lock");
                l.beneficiary = to;
                locks[to] = l;
                // emulate delete by zeroing fields
                locks[from].amount = 0;
                locks[from].release_time = 0;
            }

            // Extended functionality: clear lock for a beneficiary
            function clear_lock(address beneficiary) public {
                // emulate delete by zeroing fields
                locks[beneficiary].amount = 0;
                locks[beneficiary].release_time = 0;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let user1 = Address::generate(&runtime.env);
    let user2 = Address::generate(&runtime.env);

    let release_time: Val = 1_000_u64.into_val(&runtime.env);
    let amount: Val = 500_u64.into_val(&runtime.env);

    // Create a new lock for user1
    let create_args = vec![release_time, user1.clone().into_val(&runtime.env), amount];
    let res = runtime.invoke_contract(addr, "create_lock", create_args);
    assert!(amount.shallow_eq(&res));

    // Verify getters
    let get_amt_args = vec![user1.clone().into_val(&runtime.env)];
    let get_rel_args = vec![user1.clone().into_val(&runtime.env)];
    let get_ben_args = vec![user1.clone().into_val(&runtime.env)];
    let got_amount = runtime.invoke_contract(addr, "get_lock_amount", get_amt_args);
    let got_release = runtime.invoke_contract(addr, "get_lock_release", get_rel_args);
    let got_beneficiary = runtime.invoke_contract(addr, "get_lock_beneficiary", get_ben_args);
    assert!(amount.shallow_eq(&got_amount));
    assert!(release_time.shallow_eq(&got_release));
    let addr_val = Address::from_val(&runtime.env, &got_beneficiary);
    assert!(addr_val.equivalent(&user1));

    // Increase amount and verify new total
    let delta: Val = 250_u64.into_val(&runtime.env);
    let inc_args = vec![user1.clone().into_val(&runtime.env), delta];
    let new_total = runtime.invoke_contract(addr, "increase_lock_amount", inc_args);
    let expected_total: Val = 750_u64.into_val(&runtime.env);
    assert!(expected_total.shallow_eq(&new_total));

    // Move lock from user1 to user2
    let move_args = vec![
        user1.clone().into_val(&runtime.env),
        user2.clone().into_val(&runtime.env),
    ];
    let _ = runtime.invoke_contract(addr, "move_lock", move_args);

    // After moving, user1 should have no lock (amount == 0)
    let zero: Val = 0_u64.into_val(&runtime.env);
    let amt_user1 = runtime.invoke_contract(
        addr,
        "get_lock_amount",
        vec![user1.clone().into_val(&runtime.env)],
    );
    assert!(zero.shallow_eq(&amt_user1));

    // And user2 should now hold the moved lock with the updated total amount
    let amt_user2 = runtime.invoke_contract(
        addr,
        "get_lock_amount",
        vec![user2.clone().into_val(&runtime.env)],
    );
    assert!(expected_total.shallow_eq(&amt_user2));

    // Beneficiary for user2's lock should be user2
    let ben_user2 = runtime.invoke_contract(
        addr,
        "get_lock_beneficiary",
        vec![user2.clone().into_val(&runtime.env)],
    );
    let ben2 = Address::from_val(&runtime.env, &ben_user2);
    assert!(ben2.equivalent(&user2));

    // Clear user2's lock and verify
    let _ = runtime.invoke_contract(
        addr,
        "clear_lock",
        vec![user2.clone().into_val(&runtime.env)],
    );
    let amt_user2_after_clear =
        runtime.invoke_contract(addr, "get_lock_amount", vec![user2.into_val(&runtime.env)]);
    assert!(zero.shallow_eq(&amt_user2_after_clear));
}

// Removed: keep only two tests as requested

#[test]
fn get_whole_struct() {
    let runtime = build_solidity(
        r#"
        contract locker {
            struct Lock {
                uint64 release_time;
                address beneficiary;
                uint64 amount;
            }

            mapping(address => Lock) locks;

            function create_lock(
                uint64 release_time,
                address beneficiary,
                uint64 amount
            ) public returns (uint64) {
                Lock memory l = Lock({
                    release_time: release_time,
                    beneficiary: beneficiary,
                    amount: amount
                });

                locks[beneficiary] = l;
                return l.amount;
            }

            function get_lock_amount(address beneficiary) public view returns (uint64) {
                return locks[beneficiary].amount;
            }

            function get_lock_release(address beneficiary) public view returns (uint64) {
                return locks[beneficiary].release_time;
            }

            function get_lock_beneficiary(address key) public view returns (address) {
                return locks[key].beneficiary;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let user = Address::generate(&runtime.env);
    let release_time: Val = 42_u64.into_val(&runtime.env);
    let amount: Val = 7_u64.into_val(&runtime.env);

    // Create lock
    let _ = runtime.invoke_contract(
        addr,
        "create_lock",
        vec![release_time, user.clone().into_val(&runtime.env), amount],
    );

    // Retrieve each field via accessors (no multiple returns)
    let rt_val = runtime.invoke_contract(
        addr,
        "get_lock_release",
        vec![user.clone().into_val(&runtime.env)],
    );
    let ben_val = runtime.invoke_contract(
        addr,
        "get_lock_beneficiary",
        vec![user.clone().into_val(&runtime.env)],
    );
    let amt_val = runtime.invoke_contract(
        addr,
        "get_lock_amount",
        vec![user.clone().into_val(&runtime.env)],
    );

    let rt: u64 = FromVal::from_val(&runtime.env, &rt_val);
    let ben = Address::from_val(&runtime.env, &ben_val);
    let amt: u64 = FromVal::from_val(&runtime.env, &amt_val);

    assert_eq!(rt, 42);
    assert!(ben.equivalent(&user));
    assert_eq!(amt, 7);
}

// Step 4 (encode only): a function that RETURNS a struct must produce a named-field SCV_MAP the
// Rust SDK can decode. No storage variables. Uses non-lex field order to catch a field-swap.
#[test]
fn abi_struct_return_encodes_as_map() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct Rec {
                uint64 zebra;
                uint64 apple;
            }

            function make(uint64 z, uint64 a) public pure returns (Rec memory) {
                return Rec({ zebra: z, apple: a });
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let z: Val = 7_u64.into_val(&runtime.env);
    let a: Val = 9_u64.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "make", vec![z, a]);

    // Decode the returned map with the SDK oracle; field values must land under the right names.
    let got = Rec::from_val(&runtime.env, &res);
    assert_eq!(got, Rec { zebra: 7, apple: 9 });
}

// Encode a struct of signed integers (int32 / int64 / int128). Field names a/b/c are declared
// in a scrambled order (b, c, a) to confirm entries are keyed by name, not position. The int128
// value is in the object range (> 2^64) to exercise the Step-3 decode fix on the param side too.
#[test]
fn abi_struct_return_mixed_integers() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S {
                int64 b;
                int128 c;
                int32 a;
            }

            function make(int32 a, int64 b, int128 c) public pure returns (S memory) {
                return S({ a: a, b: b, c: c });
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let a: Val = (-7i32).into_val(&runtime.env);
    let b: Val = (-9000i64).into_val(&runtime.env);
    let c: Val = 100_000_000_000_000_000_000i128.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "make", vec![a, b, c]);

    let got = IntRec::from_val(&runtime.env, &res);
    assert_eq!(
        got,
        IntRec {
            a: -7,
            b: -9000,
            c: 100_000_000_000_000_000_000i128,
        }
    );
}

// Encode a struct of value-type fields: bool / uint32 / bytesN. Field names a/b/c declared in
// a scrambled order (c, a, b). Values built inside the contract (no param decode needed).
// NOTE: string / dynamic bytes / nested-struct fields are reference types and are NOT covered
// here — encoding them from a returned struct is not yet supported (see plan Step 4 notes).
#[test]
fn abi_struct_return_value_types() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct S {
                bytes4 c;
                bool a;
                uint32 b;
            }

            function make() public pure returns (S memory) {
                bool a = true;
                uint32 b = 42;
                bytes4 c = 0xDEADBEEF;
                return S({ a: a, b: b, c: c });
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let res = runtime.invoke_contract(addr, "make", vec![]);
    let got = ValRec::from_val(&runtime.env, &res);

    assert!(got.a);
    assert_eq!(got.b, 42);
    assert_eq!(
        got.c,
        Bytes::from_array(&runtime.env, &[0xDE, 0xAD, 0xBE, 0xEF])
    );
}

// Step 5 (decode only): a function that RECEIVES a struct param must read the incoming SCV_MAP
// by field-name key. The Solidity struct declares its fields reversed (y, x) vs the SDK oracle
// (x, y); a position-based decoder would swap them and break the sum.
#[test]
fn abi_struct_decode_param_sum() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct P {
                uint64 y;
                uint64 x;
            }

            function sum(P memory p) public pure returns (uint64) {
                return p.x + p.y;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let arg: Val = Point { x: 3, y: 4 }.into_val(&runtime.env);
    let res = runtime.invoke_contract(addr, "sum", vec![arg]);

    let got: u64 = FromVal::from_val(&runtime.env, &res);
    assert_eq!(got, 7);
}

// Step 5 round-trip: decode a struct param and re-encode it as the return value. Rec's fields
// (zebra, apple) are declared in non-lex order; `from_val` equality proves both directions keep
// each value under the right name.
#[test]
fn abi_struct_roundtrip_value_types() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct Rec {
                uint64 zebra;
                uint64 apple;
            }

            function echo(Rec memory r) public pure returns (Rec memory) {
                return r;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let input = Rec {
        zebra: 11,
        apple: 22,
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(&runtime.env)]);

    let got = Rec::from_val(&runtime.env, &res);
    assert_eq!(got, input);
}

// Step 5 + Step 3: round-trip a struct with 128-bit fields. `big` is in the object range
// (> 2^64) and `neg` is negative, so both the decode (map_get → decode_i128 with the resolved
// target type) and encode paths are exercised for signed and unsigned 128-bit struct fields.
#[test]
fn abi_struct_roundtrip_wide_ints() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct Wide {
                uint128 big;
                int128 neg;
            }

            function echo(Wide memory w) public pure returns (Wide memory) {
                return w;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let input = Wide {
        big: 100_000_000_000_000_000_000u128,
        neg: -123_456_789_012_345i128,
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(&runtime.env)]);

    let got = Wide::from_val(&runtime.env, &res);
    assert_eq!(got, input);
}

// Step 5 + Step 3: round-trip a struct with a uint256 field, guarding the decode_i256 struct-field
// path (map_get → decode_i256 with the resolved target type). The value is in the OBJECT range
// (> 2^64) so it is a U256Object handle; decode_i256 calls the ObjToU256* host ops, which require
// an object. (Inline U256Small decode is a separate, pre-existing gap not touched by Step 5.)
#[test]
fn abi_struct_roundtrip_u256_field() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct B {
                uint256 v;
            }

            function echo(B memory b) public pure returns (B memory) {
                return b;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let input = Big256 {
        v: U256::from_u128(&runtime.env, 100_000_000_000_000_000_000u128),
    };
    let res = runtime.invoke_contract(addr, "echo", vec![input.clone().into_val(&runtime.env)]);

    let got = Big256::from_val(&runtime.env, &res);
    assert_eq!(got, input);
}

// Step 5 nested decode (map of maps): a struct field that is itself a struct must recurse through
// decode_struct_map. Decode-only (nested-struct *encode* is not yet supported), so the contract
// reads fields from the two nested Points and returns a scalar.
#[test]
fn abi_struct_decode_nested() {
    let runtime = build_solidity(
        r#"
        contract test {
            struct Point {
                uint64 x;
                uint64 y;
            }

            struct Line {
                Point a;
                Point b;
            }

            function span(Line memory l) public pure returns (uint64) {
                return l.a.x + l.b.y;
            }
        }
        "#,
        |_| {},
    );

    let addr = runtime.contracts.last().unwrap();

    let input = Line {
        a: Point { x: 3, y: 4 },
        b: Point { x: 5, y: 6 },
    };
    let res = runtime.invoke_contract(addr, "span", vec![input.into_val(&runtime.env)]);

    // l.a.x (3) + l.b.y (6) == 9
    let got: u64 = FromVal::from_val(&runtime.env, &res);
    assert_eq!(got, 9);
}

// Read every `ScSpecEntry` from the emitted wasm `contractspecv0` custom section(s). LLVM may emit
// one section per entry or a single merged section; reading each section's bytes as an XDR stream
// handles both.
fn collect_spec_entries(src: &str) -> Vec<soroban_sdk::xdr::ScSpecEntry> {
    use soroban_sdk::xdr::{Limited, Limits, ReadXdr, ScSpecEntry};
    use wasmparser::{Parser, Payload};

    let (wasm, _ns) = crate::build_wasm(src);

    let mut entries = Vec::new();
    for payload in Parser::new(0)
        .parse_all(&wasm)
        .map(|p| p.expect("valid wasm payload"))
    {
        if let Payload::CustomSection(reader) = payload {
            if reader.name() == "contractspecv0" {
                let mut cursor = Limited::new(std::io::Cursor::new(reader.data()), Limits::none());
                for entry in ScSpecEntry::read_xdr_iter(&mut cursor) {
                    entries.push(entry.expect("valid ScSpecEntry XDR"));
                }
            }
        }
    }
    entries
}

fn find_udt<'a>(
    entries: &'a [soroban_sdk::xdr::ScSpecEntry],
    name: &str,
) -> Option<&'a soroban_sdk::xdr::ScSpecUdtStructV0> {
    entries.iter().find_map(|e| match e {
        soroban_sdk::xdr::ScSpecEntry::UdtStructV0(s) if s.name.to_utf8_string_lossy() == name => {
            Some(s)
        }
        _ => None,
    })
}

fn count_udt(entries: &[soroban_sdk::xdr::ScSpecEntry], name: &str) -> usize {
    entries
        .iter()
        .filter(|e| {
            matches!(e, soroban_sdk::xdr::ScSpecEntry::UdtStructV0(s)
                if s.name.to_utf8_string_lossy() == name)
        })
        .count()
}

fn find_fn<'a>(
    entries: &'a [soroban_sdk::xdr::ScSpecEntry],
    name: &str,
) -> Option<&'a soroban_sdk::xdr::ScSpecFunctionV0> {
    entries.iter().find_map(|e| match e {
        soroban_sdk::xdr::ScSpecEntry::FunctionV0(f) if f.name.to_utf8_string_lossy() == name => {
            Some(f)
        }
        _ => None,
    })
}

// Step 6: the emitted contract spec must carry an `ScSpecEntry::UdtStructV0` per struct used in a
// public signature — deduped, and including nested structs reached through a struct field — and the
// function signatures must reference the struct via `ScSpecTypeDef::Udt` (not the old `Void`).
#[test]
fn abi_struct_spec_emits_udt() {
    use soroban_sdk::xdr::ScSpecTypeDef;

    let entries = collect_spec_entries(
        r#"
        contract test {
            struct Point { uint64 x; uint64 y; }
            struct Line { Point a; Point b; }

            function span(Line memory l) public pure returns (uint64) {
                return l.a.x + l.b.y;
            }

            function origin() public pure returns (Point memory) {
                return Point(1, 2);
            }
        }
        "#,
    );

    // Both structs (outer + nested) get exactly one UdtStructV0 entry each (dedup).
    assert_eq!(count_udt(&entries, "Point"), 1);
    assert_eq!(count_udt(&entries, "Line"), 1);

    // Point fields: x, y : U64, in declaration order.
    let point = find_udt(&entries, "Point").expect("Point UdtStructV0 entry");
    let point_fields: Vec<(String, ScSpecTypeDef)> = point
        .fields
        .iter()
        .map(|f| (f.name.to_utf8_string_lossy(), f.type_.clone()))
        .collect();
    assert_eq!(
        point_fields,
        vec![
            ("x".to_string(), ScSpecTypeDef::U64),
            ("y".to_string(), ScSpecTypeDef::U64),
        ]
    );

    // Line fields: a, b — each a nested Udt("Point").
    let line = find_udt(&entries, "Line").expect("Line UdtStructV0 entry");
    let line_field_names: Vec<String> = line
        .fields
        .iter()
        .map(|f| f.name.to_utf8_string_lossy())
        .collect();
    assert_eq!(line_field_names, vec!["a".to_string(), "b".to_string()]);
    for f in line.fields.iter() {
        assert!(
            matches!(&f.type_, ScSpecTypeDef::Udt(u) if u.name.to_utf8_string_lossy() == "Point"),
            "Line.{} should be Udt(Point), got {:?}",
            f.name.to_utf8_string_lossy(),
            f.type_,
        );
    }

    // span(Line) input references Udt("Line"); its return is U64.
    let span = find_fn(&entries, "span").expect("span FunctionV0 entry");
    assert!(
        matches!(&span.inputs.first().expect("span input").type_,
            ScSpecTypeDef::Udt(u) if u.name.to_utf8_string_lossy() == "Line"),
        "span input should be Udt(Line), got {:?}",
        span.inputs.first().map(|i| &i.type_),
    );
    assert!(matches!(
        span.outputs.first().expect("span output"),
        ScSpecTypeDef::U64
    ));

    // origin() return references Udt("Point").
    let origin = find_fn(&entries, "origin").expect("origin FunctionV0 entry");
    assert!(
        matches!(origin.outputs.first().expect("origin output"),
            ScSpecTypeDef::Udt(u) if u.name.to_utf8_string_lossy() == "Point"),
        "origin output should be Udt(Point), got {:?}",
        origin.outputs.first(),
    );
}
