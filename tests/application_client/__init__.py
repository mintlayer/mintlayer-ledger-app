#  Copyright (c) 2025-2026 RBB S.r.l
#  opensource@mintlayer.org
#  SPDX-License-Identifier: MIT
#  Licensed under the MIT License;
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at
#
#  https://github.com/mintlayer/mintlayer-core/blob/master/LICENSE
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.

import scalecodec  # type: ignore

MAINNET = 0
TESTNET = 1
REGTEST = 2
SIGNET = 3


def init_mintlayer_types():
    custom_types = {
        "types": {
            "Bip32Path": "Vec<u32>",
            "Amount": "Compact<u128>",
            "H256": "[u8; 32]",
            "BlockHeight": "Compact<u64>",
            "OutputValue": {
                "type": "enum",
                "type_mapping": [
                    ["Coin", "Amount"],
                    # Note: need to have this variant to make sure TokenV1's index is 2.
                    # FIXME: the protocol should not use types from mintlayer core primitives.
                    ["DeprecatedTokenV0", ""],
                    ["TokenV1", "(TokenId, Amount)"],
                ],
            },
            "InputAddressPath": {
                "type": "struct",
                "type_mapping": [
                    ["path", "Vec<u32>"],
                    ["multisig_idx", "Option<u32>"],
                ],
            },
            "Destination": {
                "type": "enum",
                "type_mapping": [
                    ["AnyoneCanSpend", "()"],
                    ["Address", "(PublicKeyHash)"],
                    ["PublicKey", "PublicKey"],
                    ["ScriptHash", "ScriptId"],
                    ["ClassicMultiSig", "(PublicKeyHash)"],
                ],
            },
            "PublicKeyHash": "[u8; 20]",
            "PublicKey": {
                "type": "struct",
                "type_mapping": [
                    ["key", "PublicKeyHolder"],
                ],
            },
            "PublicKeyHolder": {
                "type": "enum",
                "type_mapping": [
                    ["Secp256k1Schnorr", "(Secp256k1PublicKey)"],
                ],
            },
            "Secp256k1PublicKey": {
                "type": "struct",
                "type_mapping": [
                    ["pubkey_data", "[u8; 33]"],
                ],
            },
            "IsTokenFreezable": {
                "type": "enum",
                "type_mapping": [
                    ["No", "()"],
                    ["Yes", "()"],
                ],
            },
            "TokenTotalSupply": {
                "type": "enum",
                "type_mapping": [
                    ["Fixed", "Amount"],
                    ["Lockable", "()"],
                    ["Unlimited", "()"],
                ],
            },
            "TokenIssuanceV1": {
                "type": "struct",
                "type_mapping": [
                    ["token_ticker", "Vec<u8>"],
                    ["number_of_decimals", "u8"],
                    ["metadata_uri", "Vec<u8>"],
                    ["total_supply", "TokenTotalSupply"],
                    ["authority", "Destination"],
                    ["is_freezable", "IsTokenFreezable"],
                ],
            },
            "TokenIssuance": {
                "type": "enum",
                # The Rust enum has an explicit codec index of 1 for the V1 variant.
                # A placeholder is added for index 0 to ensure correct encoding.
                "type_mapping": [
                    ["_Unused", "()"],
                    ["V1", "TokenIssuanceV1"],
                ],
            },
            "TokenCreator": {
                "type": "struct",
                "type_mapping": [
                    ["public_key", "PublicKey"],
                ],
            },
            "Metadata": {
                "type": "struct",
                "type_mapping": [
                    ["creator", "Option<TokenCreator>"],
                    ["name", "Vec<u8>"],
                    ["description", "Vec<u8>"],
                    ["ticker", "Vec<u8>"],
                    ["icon_uri", "Vec<u8>"],
                    ["additional_metadata_uri", "Vec<u8>"],
                    ["media_uri", "Vec<u8>"],
                    ["media_hash", "Vec<u8>"],
                ],
            },
            "NftIssuanceV0": {
                "type": "struct",
                "type_mapping": [
                    ["metadata", "Metadata"],
                ],
            },
            "NftIssuance": {
                "type": "enum",
                "type_mapping": [
                    ["V0", "NftIssuanceV0"],
                ],
            },
            "TxOutput": {
                "type": "enum",
                "type_mapping": [
                    ["Transfer", "(OutputValue, Destination)"],
                    ["LockThenTransfer", "(OutputValue, Destination, OutputTimeLock)"],
                    ["Burn", "(OutputValue)"],
                    ["CreateStakePool", "(PoolId, StakePoolData)"],
                    ["ProduceBlockFromStake", "(Destination, PoolId)"],
                    ["CreateDelegationId", "(Destination, PoolId)"],
                    ["DelegateStaking", "(Amount, DelegationId)"],
                    ["IssueFungibleToken", "TokenIssuance"],
                    ["IssueNft", "(TokenId, NftIssuance, Destination)"],
                    ["DataDeposit", "Vec<u8>"],
                    ["Htlc", "(OutputValue, HashedTimelockContract)"],
                ],
            },
            "HashedTimelockContract": {
                "type": "struct",
                "type_mapping": [
                    ["secret_hash", "[u8; 20]"],
                    ["spend_key", "Destination"],
                    ["refund_timelock", "OutputTimeLock"],
                    ["refund_key", "Destination"],
                ],
            },
            "OutputTimeLock": {
                "type": "enum",
                "type_mapping": [
                    ["UntilHeight", "(BlockHeight)"],
                    ["UntilTime", "(BlockTimestamp)"],
                    ["ForBlockCount", "Compact<u64>"],
                    ["ForSeconds", "Compat<u64>"],
                ],
            },
            "PoolId": "H256",
            "DelegationId": "H256",
            "TokenId": "H256",
            "OrderId": "H256",
            "StakePoolData": {
                "type": "struct",
                "type_mapping": [
                    ["value", "Amount"],
                    ["staker", "Destination"],
                    ["vrf_public_key", "VRFPublicKey"],
                    ["decommission_key", "Destination"],
                    ["margin_ratio_per_thousand", "u16"],
                    ["cost_per_block", "Amount"],
                ],
            },
            "VRFPublicKey": {
                "type": "struct",
                "type_mapping": [
                    ["key", "VRFPublicKeyHolder"],
                ],
            },
            "VRFPublicKeyHolder": {
                "type": "enum",
                "type_mapping": [
                    ["Schnorrkel", "(SchnorrkelPublicKey)"],
                ],
            },
            "SchnorrkelPublicKey": {
                "type": "struct",
                "type_mapping": [
                    ["key", "[u8; 32]"],
                ],
            },
            "OutPointSourceId": {
                "type": "enum",
                "type_mapping": [
                    ["Transaction", "H256"],
                    ["BlockReward", "H256"],
                ],
            },
            "OutPoint": {
                "type": "struct",
                "type_mapping": [
                    ["id", "OutPointSourceId"],
                    ["index", "u32"],
                ],
            },
            "AccountNonce": "Compact<u64>",
            "IsTokenUnfreezable": {
                "type": "enum",
                "type_mapping": [
                    ["No", "()"],
                    ["Yes", "()"],
                ],
            },
            "AccountCommand": {
                "type": "enum",
                "type_mapping": [
                    ["MintTokens", "(TokenId, Amount)"],
                    ["UnmintTokens", "TokenId"],
                    ["LockTokenSupply", "TokenId"],
                    ["FreezeToken", "(TokenId, IsTokenUnfreezable)"],
                    ["UnfreezeToken", "TokenId"],
                    ["ChangeTokenAuthority", "(TokenId, Destination)"],
                    ["ConcludeOrder", "OrderId"],
                    ["FillOrder", "(OrderId, Amount, Destination)"],
                    ["ChangeTokenMetadataUri", "(TokenId, Vec<u8>)"],
                ],
            },
            "OrderAccountCommand": {
                "type": "enum",
                "type_mapping": [
                    ["FillOrder", "(OrderId, Amount)"],
                    ["FreezeOrder", "OrderId"],
                    ["ConcludeOrder", "OrderId"],
                ],
            },
            "TxInputWithAdditionalInfo": {
                "type": "enum",
                "type_mapping": [
                    ["Utxo", "(OutPoint, AdditionalUtxoInfo)"],
                    ["Account", "(AccountOutPoint)"],
                    ["AccountCommand", "(AccountNonce, AccountCommand)"],
                    [
                        "OrderAccountCommand",
                        "(OrderAccountCommand, AdditionalOrderInfo)",
                    ],
                ],
            },
            "AccountOutPoint": {
                "type": "struct",
                "type_mapping": [
                    ["nonce", "Compact<u64>"],
                    ["account", "AccountSpending"],
                ],
            },
            "AccountSpending": {
                "type": "enum",
                "type_mapping": [
                    ["Delegation", "(H256, Amount)"],
                ],
            },
            "SighashInputCommitment": {
                "type": "enum",
                "type_mapping": [
                    ["None", "()"],
                    ["Utxo", "TxOutput"],
                    ["ProduceBlockFromStakeUtxo", "(TxOutput, Amount)"],
                    ["FillOrderAccountCommand", "(OutputValue, OutputValue)"],
                    [
                        "ConcludeOrderAccountCommand",
                        "(OutputValue, OutputValue, Amount, Amount)",
                    ],
                ],
            },
            "SignTxStartReq": {
                "type": "struct",
                "type_mapping": [
                    ["coin", "u8"],
                    ["version", "u8"],
                    ["num_inputs", "u32"],
                    ["num_outputs", "u32"],
                ],
            },
            "TxInputData": {
                "type": "struct",
                "type_mapping": [
                    ["addresses", "Vec<InputAddressPath>"],
                    ["input", "TxInputWithAdditionalInfo"],
                ],
            },
            "TxInputCommitmentData": {
                "type": "struct",
                "type_mapping": [
                    ["commitment", "SighashInputCommitment"],
                ],
            },
            "AdditionalUtxoInfo": {
                "type": "enum",
                "type_mapping": [
                    ["Utxo", "TxOutput"],
                    [
                        "PoolInfo",
                        "(TxOutput, Amount)",
                    ],
                ],
            },
            "AdditionalOrderInfo": {
                "type": "struct",
                "type_mapping": [
                    ["initially_asked", "OutputValue"],
                    ["initially_given", "OutputValue"],
                    ["ask_balance", "Amount"],
                    ["give_balance", "Amount"],
                ],
            },
            "TxOutputData": {
                "type": "struct",
                "type_mapping": [
                    ["output", "TxOutput"],
                ],
            },
            "SignTxNextReq": {
                "type": "enum",
                "type_mapping": [
                    ["ProcessInput", "TxInputData"],
                    ["ProcessInputCommitment", "TxInputCommitmentData"],
                    ["ProcessOutput", "TxOutputData"],
                    ["ReturnNextSignature", "()"],
                ],
            },
            "SignatureInResponse": "[u8; 64]",
            "TxInputSignatureResponse": {
                "type": "struct",
                "type_mapping": [
                    ["signature", "SignatureInResponse"],
                    ["input_idx", "u32"],
                    ["multisig_idx", "Option<u32>"],
                    ["has_next", "bool"],
                ],
            },
            "UncompressedSecp256k1PublicKey": "[u8; 65]",
            "ChainCode": "[u8; 32]",
            "PublicKeyResponse": {
                "type": "struct",
                "type_mapping": [
                    ["public_key", "UncompressedSecp256k1PublicKey"],
                    ["chain_code", "ChainCode"],
                ],
            },
            "MsgSignatureResponse": {
                "type": "struct",
                "type_mapping": [
                    ["signature", "SignatureInResponse"],
                ],
            },
            "Response": {
                "type": "enum",
                "type_mapping": [
                    ["ExpectingNextChunk", "()"],
                    ["PublicKey", "PublicKeyResponse"],
                    ["TxSetup", "()"],
                    ["TxNext", "()"],
                    ["TxInputSignature", "TxInputSignatureResponse"],
                    ["MessageSetup", "()"],
                    ["MessageSignature", "MsgSignatureResponse"],
                    ["Pong", "()"],
                ],
            },
        }
    }

    scalecodec.base.RuntimeConfiguration().update_type_registry(custom_types)


init_mintlayer_types()
