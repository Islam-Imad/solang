// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/mint-lock
pragma solidity ^0.8.20;

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
        minter_.requireAuthForArgs(contract_, to, amount);

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
