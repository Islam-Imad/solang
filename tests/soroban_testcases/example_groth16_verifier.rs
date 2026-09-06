// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use ark_bls12_381::{Fq, Fq2, G1Affine, G2Affine};
use ark_serialize::CanonicalSerialize;
use core::str::FromStr;
use soroban_sdk::{contracttype, Bytes, Env, FromVal, IntoVal, Vec, U256};

const CONTRACT: &str = r#"
contract groth16_verifier {
    uint256 constant R =
        0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001;
    uint256 constant R_MINUS_1 = R - 1;
    struct VerificationKey {
        bytes alpha;
        bytes beta;
        bytes gamma;
        bytes delta;
        bytes[] ic;
    }

    struct Proof {
        bytes a;
        bytes b;
        bytes c;
    }

    function verify_proof(
        VerificationKey vk,
        Proof proof,
        uint256[] pub_signals
    ) public returns (bool) {
        require(pub_signals.length + 1 == vk.ic.length, "MalformedVerifyingKey");

        bytes memory vk_x = vk.ic[0];
        for (uint32 i = 0; i < pub_signals.length; i++) {
            bytes memory prod = bls12_381_g1_mul(vk.ic[i + 1], pub_signals[i]);
            vk_x = bls12_381_g1_add(vk_x, prod);
        }

        bytes memory negA = bls12_381_g1_mul(proof.a, R_MINUS_1);

        bytes[] memory vp1 = new bytes[](4);
        vp1[0] = negA;
        vp1[1] = vk.alpha;
        vp1[2] = vk_x;
        vp1[3] = proof.c;

        bytes[] memory vp2 = new bytes[](4);
        vp2[0] = proof.b;
        vp2[1] = vk.beta;
        vp2[2] = vk.gamma;
        vp2[3] = vk.delta;

        return bls12_381_pairing_check(vp1, vp2);
    }
}
"#;

const G1_SERIALIZED_SIZE: usize = 96;
const G2_SERIALIZED_SIZE: usize = 192;

const ALPHA_X: &str = "851850525556173310373115880154698084608631105506432893865500290442025919078535925294035153152030470398262539759609";
const ALPHA_Y: &str = "2637289349983507610125993281171282870664683328789064436670091381805667870657250691837988574635646688089951719927247";

const BETA_X0: &str = "1312620381151154625549413690218290437739613987001512553647554932245743783919690104921577716179019375920325686841943";
const BETA_X1: &str = "1853421227732662200477195678252233549930451033531229987959164216695698667330234953033341200627605777603511819497457";
const BETA_Y0: &str = "3215807833988244618006117550809420301978856703407297742347804415291049013404133666905173282837707341742014140541018";
const BETA_Y1: &str = "812366606879346135498483310623227330050424196838294715759414425317592599094348477520229174120664109186562798527696";

const GAMMA_X0: &str = "352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160";
const GAMMA_X1: &str = "3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758";
const GAMMA_Y0: &str = "1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905";
const GAMMA_Y1: &str = "927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582";

const DELTA_X0: &str = "2981843938988033214458466658185878126396080429969635248100956025957789319926032198626745120548947333202362392267114";
const DELTA_X1: &str = "2236695112259305382987038341098587500598216646308901956168137697892380899086228863246537938263638056666003066263342";
const DELTA_Y0: &str = "717163810166643254871951856655865822196000925757284470845197358532703820821048809982340614428800986999944933231635";
const DELTA_Y1: &str = "3496058064578305387608803828034117220735807855182872031001942587835768203820179263722136810383631418598310938506798";

const IC0_X: &str = "829685638389803071404995253486571779300247099942205634643821309129201420207693030476756893332812706176564514055395";
const IC0_Y: &str = "3455508165409829148751617737772894557887792278044850553785496869183933597103951941805834639972489587640583544390358";

const IC1_X: &str = "2645559270376031734407122278942646687260452979296081924477586893972449945444985371392950465676350735694002713633589";
const IC1_Y: &str = "2241039659097418315097403108596818813895651201896886552939297756980670248638746432560267634304593609165964274111037";

const PI_A_X: &str = "314442236668110257304682488877371582255161413673331360366570443799415414639292047869143313601702131653514009114222";
const PI_A_Y: &str = "2384632327855835824635705027009217874826122107057894594162233214798350178691568018290025994699762298534539543934607";
const PI_B_X0: &str = "428844167033934720609657613212495751617651348480870890908850335525890280786532876634895457032623422366474694342656";
const PI_B_X1: &str = "3083139526360252775789959298805261067575555607578161553873977966165446991459924053189383038704105379290158793353905";
const PI_B_Y0: &str = "1590919422794657666432683000821892403620510405626533455397042191265963587891653562867091397248216891852168698286910";
const PI_B_Y1: &str = "3617931039814164588401589536353142503544155307022467123698224064329647390280346725086550997337076315487486714327146";
const PI_C_X: &str = "3052934797502613468327963344215392478880720823583493172692775426011388142569325036386650708808320216973179639719187";
const PI_C_Y: &str = "2028185281516938724429867827057869371578022471499780916652824405212207527699373814371051328341613972789943854539597";

#[contracttype]
#[derive(Clone)]
pub struct VerificationKey {
    pub alpha: Bytes,
    pub beta: Bytes,
    pub gamma: Bytes,
    pub delta: Bytes,
    pub ic: Vec<Bytes>,
}

#[contracttype]
#[derive(Clone)]
pub struct Proof {
    pub a: Bytes,
    pub b: Bytes,
    pub c: Bytes,
}

fn g1_from_coords(env: &Env, x: &str, y: &str) -> Bytes {
    let point = G1Affine::new(Fq::from_str(x).unwrap(), Fq::from_str(y).unwrap());
    let mut buf = [0u8; G1_SERIALIZED_SIZE];
    point.serialize_uncompressed(&mut buf[..]).unwrap();
    Bytes::from_slice(env, &buf)
}

fn g2_from_coords(env: &Env, x0: &str, x1: &str, y0: &str, y1: &str) -> Bytes {
    let x = Fq2::new(Fq::from_str(x0).unwrap(), Fq::from_str(x1).unwrap());
    let y = Fq2::new(Fq::from_str(y0).unwrap(), Fq::from_str(y1).unwrap());
    let point = G2Affine::new(x, y);
    let mut buf = [0u8; G2_SERIALIZED_SIZE];
    point.serialize_uncompressed(&mut buf[..]).unwrap();
    Bytes::from_slice(env, &buf)
}

fn vk(env: &Env) -> VerificationKey {
    VerificationKey {
        alpha: g1_from_coords(env, ALPHA_X, ALPHA_Y),
        beta: g2_from_coords(env, BETA_X0, BETA_X1, BETA_Y0, BETA_Y1),
        gamma: g2_from_coords(env, GAMMA_X0, GAMMA_X1, GAMMA_Y0, GAMMA_Y1),
        delta: g2_from_coords(env, DELTA_X0, DELTA_X1, DELTA_Y0, DELTA_Y1),
        ic: soroban_sdk::vec![
            env,
            g1_from_coords(env, IC0_X, IC0_Y),
            g1_from_coords(env, IC1_X, IC1_Y)
        ],
    }
}

fn proof(env: &Env) -> Proof {
    Proof {
        a: g1_from_coords(env, PI_A_X, PI_A_Y),
        b: g2_from_coords(env, PI_B_X0, PI_B_X1, PI_B_Y0, PI_B_Y1),
        c: g1_from_coords(env, PI_C_X, PI_C_Y),
    }
}

#[test]
fn groth16_verify_valid_proof_returns_true() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let pub_signals: Vec<U256> = soroban_sdk::vec![env, U256::from_u32(env, 33)];
    let res = runtime.invoke_contract(
        addr,
        "verify_proof",
        std::vec![
            vk(env).into_val(env),
            proof(env).into_val(env),
            pub_signals.into_val(env),
        ],
    );
    let ok: bool = FromVal::from_val(env, &res);
    assert!(ok, "valid proof with public signal 33 should verify");
}

#[test]
fn groth16_verify_wrong_public_signal_returns_false() {
    let runtime = build_solidity(CONTRACT, |_| {});
    let addr = runtime.contracts.last().unwrap();
    let env = &runtime.env;

    let pub_signals: Vec<U256> = soroban_sdk::vec![env, U256::from_u32(env, 22)];
    let res = runtime.invoke_contract(
        addr,
        "verify_proof",
        std::vec![
            vk(env).into_val(env),
            proof(env).into_val(env),
            pub_signals.into_val(env),
        ],
    );
    let ok: bool = FromVal::from_val(env, &res);
    assert!(!ok, "proof with wrong public signal 22 must not verify");
}
