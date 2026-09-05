// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/deployer (the deployed contract)
pragma solidity ^0.8.20;

contract deployed {
    uint32 instance stored_value;

    constructor(uint32 value) {
        stored_value = value;
    }

    function value() public view returns (uint32) {
        return stored_value;
    }
}
