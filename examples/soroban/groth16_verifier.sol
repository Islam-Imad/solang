// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/groth16_verifier
pragma solidity ^0.8.20;

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
        require(
            pub_signals.length + 1 == vk.ic.length,
            "MalformedVerifyingKey"
        );

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
