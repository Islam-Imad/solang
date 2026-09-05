// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/deployer
pragma solidity ^0.8.20;

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
