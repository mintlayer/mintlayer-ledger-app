/*****************************************************************************
 *   Mintlayer Ledger App.
 *   (c) 2025-2026 RBB S.r.l.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *****************************************************************************/

use alloc::{boxed::Box, vec};

use strum::IntoEnumIterator as _;

use mintlayer_core_primitives::{
    AccountCommandTag, AccountSpendingTag, DestinationTag, HtlcSecretHash, NftIssuanceTag,
    NftIssuanceV0, OrderAccountCommandTag, OutPointSourceIdTag, OutputTimeLockTag, OutputValueTag,
    PublicKeyTag, SchnorrkelPublicKey, SighashInputCommitmentTag, TokenIssuanceTag,
    TokenIssuanceV1, TokenTotalSupplyTag, TxInputTag, TxOutputTag, VrfPublicKeyTag,
};

use test_utils::prelude::*;

use crate::{
    AccountCommand, AccountNonce, AccountOutPoint, AccountSpending, AdditionalOrderInfo,
    AdditionalUtxoInfo, AdditionalUtxoInfoTag, AddrType, Amount, Bip32Path, BlockHeight,
    BlockTimestamp, BlocksCount, ChainCode, CoinType, DelegationId, Destination, GenBlockId,
    GetPubKeyReq, H256, HashedTimelockContract, InputAddressPath, IsTokenFreezable,
    IsTokenUnfreezable, MsgSignatureResponse, NftIssuance, OrderAccountCommand, OrderData, OrderId,
    OutPointSourceId, OutputTimeLock, OutputValue, PerThousand, PoolId, PublicKey, PublicKeyHash,
    PublicKeyResponse, Response, ResponseTag, ScriptId, SecondsCount, Secp256k1PublicKey,
    SighashInputCommitment, SignMessageStartReq, SignTxNextReq, SignTxNextReqTag, SignTxStartReq,
    Signature, StakePoolData, TokenId, TokenIssuance, TokenTotalSupply, TransactionId,
    TransactionVersion, TxInput, TxInputCommitmentData, TxInputData, TxInputSignatureResponse,
    TxInputWithAdditionalInfo, TxInputWithAdditionalInfoTag, TxOutput, TxOutputData,
    UncompressedSecp256k1PublicKey, UtxoOutPoint, VrfPublicKey, decode_all, encode,
};

#[test_item]
fn test_response() {
    for tag in ResponseTag::iter() {
        match tag {
            ResponseTag::ExpectingNextChunk => {
                let obj = Response::ExpectingNextChunk;
                let expected_enc_data = [0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Response = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            ResponseTag::PublicKey => {
                let obj = Response::PublicKey(sample_public_key_response());
                let expected_enc_data = [
                    1, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171, 171, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205,
                    205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205,
                    205, 205,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Response = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            ResponseTag::TxSetup => {
                let obj = Response::TxSetup;
                let expected_enc_data = [2];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Response = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            ResponseTag::TxNext => {
                let obj = Response::TxNext;
                let expected_enc_data = [3];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Response = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            ResponseTag::TxInputSignature => {
                let obj = Response::TxInputSignature(sample_tx_input_signature_response());
                let expected_enc_data = [
                    4, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171, 123, 0, 0, 0, 1, 234, 0, 0, 0, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Response = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            ResponseTag::MessageSetup => {
                let obj = Response::MessageSetup;
                let expected_enc_data = [5];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Response = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            ResponseTag::MessageSignature => {
                let obj = Response::MessageSignature(sample_msg_signature_response());
                let expected_enc_data = [
                    6, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
                    171,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Response = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            ResponseTag::Pong => {
                let obj = Response::Pong;
                let expected_enc_data = [7];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Response = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_public_key_response() {
    let obj = sample_public_key_response();
    let expected_enc_data = [
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 205, 205, 205, 205, 205, 205, 205,
        205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205,
        205, 205, 205, 205, 205, 205, 205,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: PublicKeyResponse = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_tx_input_signature_response() {
    let obj = sample_tx_input_signature_response();
    let expected_enc_data = [
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 123, 0, 0, 0, 1, 234, 0, 0, 0, 1,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: TxInputSignatureResponse = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_msg_signature_response() {
    let obj = sample_msg_signature_response();
    let expected_enc_data = [
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: MsgSignatureResponse = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_get_pub_key_req() {
    let obj = sample_get_pub_key_req();
    let expected_enc_data = [0, 12, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: GetPubKeyReq = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_sign_msg_start_req() {
    let obj = sample_sign_message_start_req();
    let expected_enc_data = [0, 0, 12, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: SignMessageStartReq = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_sign_tx_start_req() {
    let obj = sample_sign_tx_start_req();
    let expected_enc_data = [0, 0, 12, 0, 0, 0, 23, 0, 0, 0];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: SignTxStartReq = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_sign_tx_next_req() {
    for tag in SignTxNextReqTag::iter() {
        match tag {
            SignTxNextReqTag::ProcessInput => {
                let obj = SignTxNextReq::ProcessInput(Box::new(sample_tx_input_data()));
                let expected_enc_data = [
                    0, 4, 12, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 1, 123, 0, 0, 0, 1, 237, 1, 0, 3,
                    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                    3, 3, 3, 3, 3, 237, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: SignTxNextReq = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            SignTxNextReqTag::ProcessInputCommitment => {
                let obj = SignTxNextReq::ProcessInputCommitment(Box::new(
                    sample_tx_input_commitment_data(),
                ));
                let expected_enc_data = [1, 1, 2, 0, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: SignTxNextReq = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            SignTxNextReqTag::ProcessOutput => {
                let obj = SignTxNextReq::ProcessOutput(Box::new(sample_tx_output_data()));
                let expected_enc_data =
                    [2, 2, 0, 237, 1, 1, 12, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: SignTxNextReq = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            SignTxNextReqTag::ReturnNextSignature => {
                let obj = SignTxNextReq::ReturnNextSignature;
                let expected_enc_data = [3];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: SignTxNextReq = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_tx_input_data() {
    let obj = sample_tx_input_data();
    let expected_enc_data = [
        4, 12, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 1, 123, 0, 0, 0, 1, 237, 1, 0, 3, 3, 3, 3, 3, 3,
        3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 237, 1,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: TxInputData = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_tx_input_commitment_data() {
    let obj = sample_tx_input_commitment_data();
    let expected_enc_data = [1, 2, 0, 237, 1];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: TxInputCommitmentData = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_tx_output_data() {
    let obj = sample_tx_output_data();
    let expected_enc_data = [2, 0, 237, 1, 1, 12, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: TxOutputData = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_tx_input_with_additional_info() {
    for tag in TxInputWithAdditionalInfoTag::iter() {
        match tag {
            TxInputWithAdditionalInfoTag::Utxo => {
                let obj = TxInputWithAdditionalInfo::Utxo(
                    sample_utxo_out_point(),
                    AdditionalUtxoInfo::Utxo(sample_tx_output()),
                );
                let expected_enc_data = [
                    0, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
                    5, 5, 5, 5, 5, 5, 5, 5, 123, 0, 0, 0, 0, 2, 0, 237, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxInputWithAdditionalInfo = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxInputWithAdditionalInfoTag::Account => {
                let obj = TxInputWithAdditionalInfo::Account(sample_account_out_point());
                let expected_enc_data = [
                    1, 237, 1, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 237, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxInputWithAdditionalInfo = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxInputWithAdditionalInfoTag::AccountCommand => {
                let obj = TxInputWithAdditionalInfo::AccountCommand(
                    AccountNonce(123),
                    sample_account_command(),
                );
                let expected_enc_data = [
                    2, 237, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                    2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxInputWithAdditionalInfo = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxInputWithAdditionalInfoTag::OrderAccountCommand => {
                let obj = TxInputWithAdditionalInfo::OrderAccountCommand(
                    sample_order_account_command(),
                    sample_additional_order_info(),
                );
                let expected_enc_data = [
                    3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1, 1, 1, 0, 237, 1, 0, 237, 1, 237, 1, 237, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxInputWithAdditionalInfo = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_additional_utxo_info() {
    for tag in AdditionalUtxoInfoTag::iter() {
        match tag {
            AdditionalUtxoInfoTag::Utxo => {
                let obj = AdditionalUtxoInfo::Utxo(sample_tx_output());
                let expected_enc_data = [0, 2, 0, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AdditionalUtxoInfo = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            AdditionalUtxoInfoTag::UtxoWithPoolData => {
                let obj = AdditionalUtxoInfo::UtxoWithPoolData {
                    utxo: sample_tx_output(),
                    staker_balance: sample_amount(),
                };
                let expected_enc_data = [1, 2, 0, 237, 1, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AdditionalUtxoInfo = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_additional_order_info() {
    let obj = sample_additional_order_info();
    let expected_enc_data = [0, 237, 1, 0, 237, 1, 237, 1, 237, 1];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: AdditionalOrderInfo = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_tx_input() {
    for tag in TxInputTag::iter() {
        match tag {
            TxInputTag::Utxo => {
                let obj = TxInput::Utxo(sample_utxo_out_point());
                let expected_enc_data = [
                    0, 0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
                    5, 5, 5, 5, 5, 5, 5, 5, 123, 0, 0, 0,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxInput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxInputTag::Account => {
                let obj = TxInput::Account(sample_account_out_point());
                let expected_enc_data = [
                    1, 237, 1, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 237, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxInput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxInputTag::AccountCommand => {
                let obj = TxInput::AccountCommand(AccountNonce(123), sample_account_command());
                let expected_enc_data = [
                    2, 237, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                    2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxInput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxInputTag::OrderAccountCommand => {
                let obj = TxInput::OrderAccountCommand(sample_order_account_command());
                let expected_enc_data = [
                    3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1, 1, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxInput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_tx_output() {
    for tag in TxOutputTag::iter() {
        match tag {
            TxOutputTag::Transfer => {
                let obj = TxOutput::Transfer(sample_output_value(), sample_destination());
                let expected_enc_data = [0, 0, 237, 1, 0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxOutputTag::LockThenTransfer => {
                let obj = TxOutput::LockThenTransfer(
                    sample_output_value(),
                    sample_destination(),
                    sample_output_time_lock(),
                );
                let expected_enc_data = [1, 0, 237, 1, 0, 2, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxOutputTag::Burn => {
                let obj = TxOutput::Burn(sample_output_value());
                let expected_enc_data = [2, 0, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxOutputTag::CreateStakePool => {
                let obj = TxOutput::CreateStakePool(sample_pool_id(), sample_stake_pool_data());
                let expected_enc_data = [
                    3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
                    4, 4, 4, 4, 4, 4, 4, 237, 1, 0, 0, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
                    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
                    10, 0, 123, 0, 237, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxOutputTag::ProduceBlockFromStake => {
                let obj = TxOutput::ProduceBlockFromStake(sample_destination(), sample_pool_id());
                let expected_enc_data = [
                    4, 0, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
                    4, 4, 4, 4, 4, 4, 4, 4,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxOutputTag::CreateDelegationId => {
                let obj = TxOutput::CreateDelegationId(sample_destination(), sample_pool_id());
                let expected_enc_data = [
                    5, 0, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
                    4, 4, 4, 4, 4, 4, 4, 4,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxOutputTag::DelegateStaking => {
                let obj = TxOutput::DelegateStaking(sample_amount(), sample_delegation_id());
                let expected_enc_data = [
                    6, 237, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                    3, 3, 3, 3, 3, 3, 3, 3, 3,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxOutputTag::IssueFungibleToken => {
                let obj = TxOutput::IssueFungibleToken(TokenIssuance::V1(
                    sample_token_issuance_v1(TokenTotalSupply::Unlimited),
                ));
                let expected_enc_data = [7, 1, 12, 1, 2, 3, 8, 12, 4, 5, 6, 2, 0, 0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxOutputTag::IssueNft => {
                let obj = TxOutput::IssueNft(
                    sample_token_id(),
                    NftIssuance::V0(sample_nft_issuance_v0(Some(sample_public_key()))),
                    sample_destination(),
                );
                let expected_enc_data = [
                    8, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                    2, 2, 2, 2, 2, 2, 2, 0, 1, 0, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
                    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 12, 1, 2, 3, 12, 4, 5, 6,
                    12, 7, 8, 9, 12, 10, 11, 12, 12, 13, 14, 15, 12, 16, 17, 18, 12, 19, 20, 21, 0,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxOutputTag::DataDeposit => {
                let obj = TxOutput::DataDeposit(vec![1, 2, 3]);
                let expected_enc_data = [9, 12, 1, 2, 3];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxOutputTag::Htlc => {
                let obj = TxOutput::Htlc(sample_output_value(), sample_hashed_timelock_contract());
                let expected_enc_data = [
                    10, 0, 237, 1, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
                    11, 11, 11, 11, 0, 2, 237, 1, 0,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TxOutputTag::CreateOrder => {
                let obj = TxOutput::CreateOrder(sample_order_data());
                let expected_enc_data = [11, 0, 0, 237, 1, 0, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TxOutput = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_sighash_input_commitment() {
    for tag in SighashInputCommitmentTag::iter() {
        match tag {
            SighashInputCommitmentTag::None => {
                let obj = SighashInputCommitment::None;
                let expected_enc_data = [0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: SighashInputCommitment = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            SighashInputCommitmentTag::Utxo => {
                let obj = SighashInputCommitment::Utxo(sample_tx_output());
                let expected_enc_data = [1, 2, 0, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: SighashInputCommitment = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            SighashInputCommitmentTag::ProduceBlockFromStakeUtxo => {
                let obj = SighashInputCommitment::ProduceBlockFromStakeUtxo {
                    utxo: sample_tx_output(),
                    staker_balance: sample_amount(),
                };
                let expected_enc_data = [2, 2, 0, 237, 1, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: SighashInputCommitment = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            SighashInputCommitmentTag::FillOrderAccountCommand => {
                let obj = SighashInputCommitment::FillOrderAccountCommand {
                    initially_asked: sample_output_value(),
                    initially_given: sample_output_value(),
                };
                let expected_enc_data = [3, 0, 237, 1, 0, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: SighashInputCommitment = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            SighashInputCommitmentTag::ConcludeOrderAccountCommand => {
                let obj = SighashInputCommitment::ConcludeOrderAccountCommand {
                    initially_asked: sample_output_value(),
                    initially_given: sample_output_value(),
                    ask_balance: sample_amount(),
                    give_balance: sample_amount(),
                };
                let expected_enc_data = [4, 0, 237, 1, 0, 237, 1, 237, 1, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: SighashInputCommitment = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_account_out_point() {
    let obj = sample_account_out_point();
    let expected_enc_data = [
        237, 1, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        3, 3, 3, 3, 3, 237, 1,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: AccountOutPoint = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_account_spending() {
    for tag in AccountSpendingTag::iter() {
        match tag {
            AccountSpendingTag::DelegationBalance => {
                let obj =
                    AccountSpending::DelegationBalance(sample_delegation_id(), sample_amount());
                let expected_enc_data = [
                    0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                    3, 3, 3, 3, 3, 3, 3, 237, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AccountSpending = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_account_command() {
    for tag in AccountCommandTag::iter() {
        match tag {
            AccountCommandTag::MintTokens => {
                let obj = AccountCommand::MintTokens(sample_token_id(), sample_amount());
                let expected_enc_data = [
                    0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                    2, 2, 2, 2, 2, 2, 2, 237, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            AccountCommandTag::UnmintTokens => {
                let obj = AccountCommand::UnmintTokens(sample_token_id());
                let expected_enc_data = [
                    1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                    2, 2, 2, 2, 2, 2, 2,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            AccountCommandTag::LockTokenSupply => {
                let obj = AccountCommand::LockTokenSupply(sample_token_id());
                let expected_enc_data = [
                    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                    2, 2, 2, 2, 2, 2, 2,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            AccountCommandTag::FreezeToken => {
                let obj = AccountCommand::FreezeToken(sample_token_id(), IsTokenUnfreezable::No);
                let expected_enc_data = [
                    3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                    2, 2, 2, 2, 2, 2, 2, 0,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            AccountCommandTag::UnfreezeToken => {
                let obj = AccountCommand::UnfreezeToken(sample_token_id());
                let expected_enc_data = [
                    4, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                    2, 2, 2, 2, 2, 2, 2,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            AccountCommandTag::ChangeTokenAuthority => {
                let obj =
                    AccountCommand::ChangeTokenAuthority(sample_token_id(), sample_destination());
                let expected_enc_data = [
                    5, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                    2, 2, 2, 2, 2, 2, 2, 0,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            AccountCommandTag::ConcludeOrder => {
                let obj = AccountCommand::ConcludeOrder(sample_order_id());
                let expected_enc_data = [
                    6, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            AccountCommandTag::FillOrder => {
                let obj = AccountCommand::FillOrder(
                    sample_order_id(),
                    sample_amount(),
                    sample_destination(),
                );
                let expected_enc_data = [
                    7, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1, 1, 237, 1, 0,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            AccountCommandTag::ChangeTokenMetadataUri => {
                let obj = AccountCommand::ChangeTokenMetadataUri(sample_token_id(), vec![1, 2, 3]);
                let expected_enc_data = [
                    8, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                    2, 2, 2, 2, 2, 2, 2, 12, 1, 2, 3,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_order_account_command() {
    for tag in OrderAccountCommandTag::iter() {
        match tag {
            OrderAccountCommandTag::FillOrder => {
                let obj = OrderAccountCommand::FillOrder(sample_order_id(), sample_amount());
                let expected_enc_data = [
                    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1, 1, 237, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: OrderAccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            OrderAccountCommandTag::FreezeOrder => {
                let obj = OrderAccountCommand::FreezeOrder(sample_order_id());
                let expected_enc_data = [
                    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: OrderAccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            OrderAccountCommandTag::ConcludeOrder => {
                let obj = OrderAccountCommand::ConcludeOrder(sample_order_id());
                let expected_enc_data = [
                    2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: OrderAccountCommand = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_utxo_out_point() {
    let obj = sample_utxo_out_point();
    let expected_enc_data = [
        0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 123, 0, 0, 0,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: UtxoOutPoint = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_out_point_source_id() {
    for tag in OutPointSourceIdTag::iter() {
        match tag {
            OutPointSourceIdTag::Transaction => {
                let obj = OutPointSourceId::Transaction(sample_transaction_id());
                let expected_enc_data = [
                    0, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
                    5, 5, 5, 5, 5, 5, 5,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: OutPointSourceId = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            OutPointSourceIdTag::BlockReward => {
                let obj = OutPointSourceId::BlockReward(sample_gen_block_id());
                let expected_enc_data = [
                    1, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
                    6, 6, 6, 6, 6, 6, 6,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: OutPointSourceId = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_stake_pool_data() {
    let obj = sample_stake_pool_data();
    let expected_enc_data = [
        237, 1, 0, 0, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 0, 123, 0, 237, 1,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: StakePoolData = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_order_data() {
    let obj = sample_order_data();
    let expected_enc_data = [0, 0, 237, 1, 0, 237, 1];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: OrderData = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_hashed_timelock_contract() {
    let obj = sample_hashed_timelock_contract();
    let expected_enc_data = [
        11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 0, 2, 237,
        1, 0,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: HashedTimelockContract = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_token_issuance() {
    for tag in TokenIssuanceTag::iter() {
        match tag {
            TokenIssuanceTag::V1 => {
                let obj = TokenIssuance::V1(sample_token_issuance_v1(TokenTotalSupply::Unlimited));
                let expected_enc_data = [1, 12, 1, 2, 3, 8, 12, 4, 5, 6, 2, 0, 0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TokenIssuance = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_token_issuance_v1() {
    let obj = sample_token_issuance_v1(TokenTotalSupply::Unlimited);
    let expected_enc_data = [12, 1, 2, 3, 8, 12, 4, 5, 6, 2, 0, 0];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: TokenIssuanceV1 = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_token_total_supply() {
    for tag in TokenTotalSupplyTag::iter() {
        match tag {
            TokenTotalSupplyTag::Fixed => {
                let obj = TokenTotalSupply::Fixed(sample_amount());
                let expected_enc_data = [0, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TokenTotalSupply = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TokenTotalSupplyTag::Lockable => {
                let obj = TokenTotalSupply::Lockable;
                let expected_enc_data = [1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TokenTotalSupply = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            TokenTotalSupplyTag::Unlimited => {
                let obj = TokenTotalSupply::Unlimited;
                let expected_enc_data = [2];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TokenTotalSupply = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_nft_issuance() {
    for tag in NftIssuanceTag::iter() {
        match tag {
            NftIssuanceTag::V0 => {
                let obj = NftIssuance::V0(sample_nft_issuance_v0(Some(sample_public_key())));
                let expected_enc_data = [
                    0, 1, 0, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
                    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 12, 1, 2, 3, 12, 4, 5, 6, 12, 7, 8, 9, 12, 10,
                    11, 12, 12, 13, 14, 15, 12, 16, 17, 18, 12, 19, 20, 21,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: NftIssuance = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_nft_issuance_v0() {
    let obj = sample_nft_issuance_v0(Some(sample_public_key()));
    let expected_enc_data = [
        1, 0, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
        9, 9, 9, 9, 9, 12, 1, 2, 3, 12, 4, 5, 6, 12, 7, 8, 9, 12, 10, 11, 12, 12, 13, 14, 15, 12,
        16, 17, 18, 12, 19, 20, 21,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: NftIssuanceV0 = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_destination() {
    for tag in DestinationTag::iter() {
        match tag {
            DestinationTag::AnyoneCanSpend => {
                let obj = Destination::AnyoneCanSpend;
                let expected_enc_data = [0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Destination = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            DestinationTag::PublicKeyHash => {
                let obj = Destination::PublicKeyHash(sample_public_key_hash());
                let expected_enc_data = [
                    1, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Destination = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            DestinationTag::PublicKey => {
                let obj = Destination::PublicKey(sample_public_key());
                let expected_enc_data = [
                    2, 0, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
                    9, 9, 9, 9, 9, 9, 9, 9, 9,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Destination = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            DestinationTag::ScriptHash => {
                let obj = Destination::ScriptHash(sample_script_id());
                let expected_enc_data = [
                    3, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
                    7, 7, 7, 7, 7, 7, 7,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Destination = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            DestinationTag::ClassicMultisig => {
                let obj = Destination::ClassicMultisig(sample_public_key_hash());
                let expected_enc_data = [
                    4, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: Destination = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_public_key() {
    for tag in PublicKeyTag::iter() {
        match tag {
            PublicKeyTag::Secp256k1Schnorr => {
                let obj = PublicKey::Secp256k1Schnorr(sample_secp256k1_public_key());
                let expected_enc_data = [
                    0, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
                    9, 9, 9, 9, 9, 9, 9, 9,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: PublicKey = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_vrf_public_key() {
    for tag in VrfPublicKeyTag::iter() {
        match tag {
            VrfPublicKeyTag::Schnorrkel => {
                let obj = VrfPublicKey::Schnorrkel(sample_schnorrkel_public_key());
                let expected_enc_data = [
                    0, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
                    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: VrfPublicKey = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_output_value() {
    for tag in OutputValueTag::iter() {
        match tag {
            OutputValueTag::Coin => {
                let obj = OutputValue::Coin(sample_amount());
                let expected_enc_data = [0, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: OutputValue = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            OutputValueTag::TokenV1 => {
                let obj = OutputValue::TokenV1(sample_token_id(), sample_amount());
                let expected_enc_data = [
                    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                    2, 2, 2, 2, 2, 2, 2, 237, 1,
                ];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: OutputValue = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_output_time_lock() {
    for tag in OutputTimeLockTag::iter() {
        match tag {
            OutputTimeLockTag::UntilHeight => {
                let obj = OutputTimeLock::UntilHeight(BlockHeight(123));
                let expected_enc_data = [0, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: OutputTimeLock = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            OutputTimeLockTag::UntilTime => {
                let obj = OutputTimeLock::UntilTime(BlockTimestamp(SecondsCount(123)));
                let expected_enc_data = [1, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: OutputTimeLock = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            OutputTimeLockTag::ForBlockCount => {
                let obj = OutputTimeLock::ForBlockCount(BlocksCount(123));
                let expected_enc_data = [2, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: OutputTimeLock = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            OutputTimeLockTag::ForSeconds => {
                let obj = OutputTimeLock::ForSeconds(SecondsCount(123));
                let expected_enc_data = [3, 237, 1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: OutputTimeLock = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_input_address_path() {
    let obj = sample_input_address_path();
    let expected_enc_data = [12, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 1, 123, 0, 0, 0];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: InputAddressPath = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_bip32_path() {
    let obj = sample_bip32_path();
    let expected_enc_data = [12, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: Bip32Path = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_coin_type() {
    for coin_type in CoinType::iter() {
        match coin_type {
            CoinType::Mainnet => {
                let obj = CoinType::Mainnet;
                let expected_enc_data = [0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: CoinType = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            CoinType::Testnet => {
                let obj = CoinType::Testnet;
                let expected_enc_data = [1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: CoinType = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            CoinType::Regtest => {
                let obj = CoinType::Regtest;
                let expected_enc_data = [2];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: CoinType = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            CoinType::Signet => {
                let obj = CoinType::Signet;
                let expected_enc_data = [3];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: CoinType = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_addr_type() {
    for addr_type in AddrType::iter() {
        match addr_type {
            AddrType::PublicKey => {
                let obj = AddrType::PublicKey;
                let expected_enc_data = [0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AddrType = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            AddrType::PublicKeyHash => {
                let obj = AddrType::PublicKeyHash;
                let expected_enc_data = [1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: AddrType = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_tx_version() {
    for version in TransactionVersion::iter() {
        match version {
            TransactionVersion::V1 => {
                let obj = TransactionVersion::V1;
                let expected_enc_data = [0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: TransactionVersion = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_uncompressed_secp256k1_public_key() {
    let obj = UncompressedSecp256k1PublicKey([0xAB; 65]);
    let expected_enc_data = [
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: UncompressedSecp256k1PublicKey = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_chain_code() {
    let obj = ChainCode([0xCD; 32]);
    let expected_enc_data = [
        205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205,
        205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205, 205,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: ChainCode = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_signature() {
    let obj = sample_signature();
    let expected_enc_data = [
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
        171, 171, 171, 171, 171, 171, 171, 171, 171, 171,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: Signature = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_htlc_secret_hash() {
    let obj: HtlcSecretHash = [0x0B; 20].into();
    let expected_enc_data = [
        11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: HtlcSecretHash = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_per_thousand() {
    let obj = PerThousand(123);
    let expected_enc_data = [123, 0];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: PerThousand = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_public_key_hash() {
    let obj = sample_public_key_hash();
    let expected_enc_data = [8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: PublicKeyHash = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_secp256k1_public_key() {
    let obj = sample_secp256k1_public_key();
    let expected_enc_data = [
        9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
        9, 9, 9,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: Secp256k1PublicKey = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_schnorrkel_public_key() {
    let obj = sample_schnorrkel_public_key();
    let expected_enc_data = [
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: SchnorrkelPublicKey = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_h256() {
    let obj = sample_h256(0x01);
    let expected_enc_data = [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: H256 = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_order_id() {
    let obj = sample_order_id();
    let expected_enc_data = [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: OrderId = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_token_id() {
    let obj = sample_token_id();
    let expected_enc_data = [
        2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        2, 2,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: TokenId = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_delegation_id() {
    let obj = sample_delegation_id();
    let expected_enc_data = [
        3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        3, 3,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: DelegationId = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_pool_id() {
    let obj = sample_pool_id();
    let expected_enc_data = [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: PoolId = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_transaction_id() {
    let obj = sample_transaction_id();
    let expected_enc_data = [
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: TransactionId = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_gen_block_id() {
    let obj = sample_gen_block_id();
    let expected_enc_data = [
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: GenBlockId = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_script_id() {
    let obj = sample_script_id();
    let expected_enc_data = [
        7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        7, 7,
    ];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, &expected_enc_data);
    let decoded_obj: ScriptId = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_is_token_freezable() {
    for freezable in IsTokenFreezable::iter() {
        match freezable {
            IsTokenFreezable::No => {
                let obj = IsTokenFreezable::No;
                let expected_enc_data = [0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: IsTokenFreezable = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            IsTokenFreezable::Yes => {
                let obj = IsTokenFreezable::Yes;
                let expected_enc_data = [1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: IsTokenFreezable = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_is_token_unfreezable() {
    for unfreezable in IsTokenUnfreezable::iter() {
        match unfreezable {
            IsTokenUnfreezable::No => {
                let obj = IsTokenUnfreezable::No;
                let expected_enc_data = [0];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: IsTokenUnfreezable = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
            IsTokenUnfreezable::Yes => {
                let obj = IsTokenUnfreezable::Yes;
                let expected_enc_data = [1];

                let enc_data = encode(&obj);
                assert_eq!(enc_data, &expected_enc_data);
                let decoded_obj: IsTokenUnfreezable = decode_all(&enc_data).unwrap();
                assert_eq!(decoded_obj, obj);
            }
        }
    }
}

#[test_item]
fn test_block_timestamp() {
    let obj = BlockTimestamp(SecondsCount(123));
    let expected_enc_data = [237, 1];

    let enc_data = encode(&obj);
    assert_eq!(enc_data, expected_enc_data);
    let decoded_obj: BlockTimestamp = decode_all(&enc_data).unwrap();
    assert_eq!(decoded_obj, obj);
}

#[test_item]
fn test_account_nonce() {
    let tests = [
        (AccountNonce(0), vec![0]),
        (AccountNonce(1_u64 << 6), vec![1, 1]),
        (AccountNonce(1_u64 << 14), vec![2, 0, 1, 0]),
        (AccountNonce(1_u64 << 30), vec![3, 0, 0, 0, 64]),
        (AccountNonce(1_u64 << 32), vec![7, 0, 0, 0, 0, 1]),
        (AccountNonce(1_u64 << 40), vec![11, 0, 0, 0, 0, 0, 1]),
        (AccountNonce(1_u64 << 48), vec![15, 0, 0, 0, 0, 0, 0, 1]),
        (AccountNonce(1_u64 << 56), vec![19, 0, 0, 0, 0, 0, 0, 0, 1]),
        (
            AccountNonce(1_u64 << 63),
            vec![19, 0, 0, 0, 0, 0, 0, 0, 128],
        ),
    ];

    for (obj, expected_enc_data) in tests {
        let enc_data = encode(&obj);
        assert_eq!(enc_data, expected_enc_data);
        let decoded_obj: AccountNonce = decode_all(&enc_data).unwrap();
        assert_eq!(decoded_obj, obj);
    }
}

#[test_item]
fn test_block_height() {
    let tests = [
        (BlockHeight(0), vec![0]),
        (BlockHeight(1_u64 << 6), vec![1, 1]),
        (BlockHeight(1_u64 << 14), vec![2, 0, 1, 0]),
        (BlockHeight(1_u64 << 30), vec![3, 0, 0, 0, 64]),
        (BlockHeight(1_u64 << 32), vec![7, 0, 0, 0, 0, 1]),
        (BlockHeight(1_u64 << 40), vec![11, 0, 0, 0, 0, 0, 1]),
        (BlockHeight(1_u64 << 48), vec![15, 0, 0, 0, 0, 0, 0, 1]),
        (BlockHeight(1_u64 << 56), vec![19, 0, 0, 0, 0, 0, 0, 0, 1]),
        (BlockHeight(1_u64 << 63), vec![19, 0, 0, 0, 0, 0, 0, 0, 128]),
    ];

    for (obj, expected_enc_data) in tests {
        let enc_data = encode(&obj);
        assert_eq!(enc_data, expected_enc_data);
        let decoded_obj: BlockHeight = decode_all(&enc_data).unwrap();
        assert_eq!(decoded_obj, obj);
    }
}

#[test_item]
fn test_blocks_count() {
    let tests = [
        (BlocksCount(0), vec![0]),
        (BlocksCount(1_u64 << 6), vec![1, 1]),
        (BlocksCount(1_u64 << 14), vec![2, 0, 1, 0]),
        (BlocksCount(1_u64 << 30), vec![3, 0, 0, 0, 64]),
        (BlocksCount(1_u64 << 32), vec![7, 0, 0, 0, 0, 1]),
        (BlocksCount(1_u64 << 40), vec![11, 0, 0, 0, 0, 0, 1]),
        (BlocksCount(1_u64 << 48), vec![15, 0, 0, 0, 0, 0, 0, 1]),
        (BlocksCount(1_u64 << 56), vec![19, 0, 0, 0, 0, 0, 0, 0, 1]),
        (BlocksCount(1_u64 << 63), vec![19, 0, 0, 0, 0, 0, 0, 0, 128]),
    ];

    for (obj, expected_enc_data) in tests {
        let enc_data = encode(&obj);
        assert_eq!(enc_data, expected_enc_data);
        let decoded_obj: BlocksCount = decode_all(&enc_data).unwrap();
        assert_eq!(decoded_obj, obj);
    }
}

#[test_item]
fn test_seconds_count() {
    let tests = [
        (SecondsCount(0), vec![0]),
        (SecondsCount(1_u64 << 6), vec![1, 1]),
        (SecondsCount(1_u64 << 14), vec![2, 0, 1, 0]),
        (SecondsCount(1_u64 << 30), vec![3, 0, 0, 0, 64]),
        (SecondsCount(1_u64 << 32), vec![7, 0, 0, 0, 0, 1]),
        (SecondsCount(1_u64 << 40), vec![11, 0, 0, 0, 0, 0, 1]),
        (SecondsCount(1_u64 << 48), vec![15, 0, 0, 0, 0, 0, 0, 1]),
        (SecondsCount(1_u64 << 56), vec![19, 0, 0, 0, 0, 0, 0, 0, 1]),
        (
            SecondsCount(1_u64 << 63),
            vec![19, 0, 0, 0, 0, 0, 0, 0, 128],
        ),
    ];

    for (obj, expected_enc_data) in tests {
        let enc_data = encode(&obj);
        assert_eq!(enc_data, expected_enc_data);
        let decoded_obj: SecondsCount = decode_all(&enc_data).unwrap();
        assert_eq!(decoded_obj, obj);
    }
}

#[test_item]
fn test_amount() {
    let tests = [
        (Amount::from_atoms(0), vec![0]),
        (Amount::from_atoms(1_u128 << 6), vec![1, 1]),
        (Amount::from_atoms(1_u128 << 14), vec![2, 0, 1, 0]),
        (Amount::from_atoms(1_u128 << 30), vec![3, 0, 0, 0, 64]),
        (Amount::from_atoms(1_u128 << 32), vec![7, 0, 0, 0, 0, 1]),
        (Amount::from_atoms(1_u128 << 40), vec![11, 0, 0, 0, 0, 0, 1]),
        (
            Amount::from_atoms(1_u128 << 48),
            vec![15, 0, 0, 0, 0, 0, 0, 1],
        ),
        (
            Amount::from_atoms(1_u128 << 56),
            vec![19, 0, 0, 0, 0, 0, 0, 0, 1],
        ),
        (
            Amount::from_atoms(1_u128 << 64),
            vec![23, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        ),
        (
            Amount::from_atoms(1_u128 << 72),
            vec![27, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        ),
        (
            Amount::from_atoms(1_u128 << 80),
            vec![31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        ),
        (
            Amount::from_atoms(1_u128 << 88),
            vec![35, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        ),
        (
            Amount::from_atoms(1_u128 << 96),
            vec![39, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        ),
        (
            Amount::from_atoms(1_u128 << 104),
            vec![43, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        ),
        (
            Amount::from_atoms(1_u128 << 112),
            vec![47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        ),
        (
            Amount::from_atoms(1_u128 << 120),
            vec![51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        ),
        (
            Amount::from_atoms(1_u128 << 127),
            vec![51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128],
        ),
    ];

    for (obj, expected_enc_data) in tests {
        let enc_data = encode(&obj);
        assert_eq!(enc_data, expected_enc_data);
        let decoded_obj: Amount = decode_all(&enc_data).unwrap();
        assert_eq!(decoded_obj, obj);
    }
}

fn sample_get_pub_key_req() -> GetPubKeyReq {
    GetPubKeyReq {
        coin_type: CoinType::Mainnet,
        path: sample_bip32_path(),
    }
}

fn sample_sign_message_start_req() -> SignMessageStartReq {
    SignMessageStartReq {
        coin_type: CoinType::Mainnet,
        addr_type: AddrType::PublicKey,
        path: sample_bip32_path(),
    }
}

fn sample_sign_tx_start_req() -> SignTxStartReq {
    SignTxStartReq {
        coin_type: CoinType::Mainnet,
        version: TransactionVersion::V1,
        num_inputs: 12,
        num_outputs: 23,
    }
}

fn sample_tx_input_data() -> TxInputData {
    TxInputData {
        addresses: vec![sample_input_address_path()],
        input: sample_tx_input_with_additional_info(),
    }
}

fn sample_tx_input_commitment_data() -> TxInputCommitmentData {
    TxInputCommitmentData {
        commitment: SighashInputCommitment::Utxo(sample_tx_output()),
    }
}

fn sample_tx_output_data() -> TxOutputData {
    TxOutputData {
        output: sample_tx_output(),
        change_path: Some(sample_bip32_path()),
    }
}

fn sample_public_key_response() -> PublicKeyResponse {
    PublicKeyResponse {
        public_key: UncompressedSecp256k1PublicKey([0xAB; 65]),
        chain_code: ChainCode([0xCD; 32]),
    }
}

fn sample_tx_input_signature_response() -> TxInputSignatureResponse {
    TxInputSignatureResponse {
        signature: sample_signature(),
        input_idx: 123,
        multisig_idx: Some(234),
        has_next: true,
    }
}

fn sample_msg_signature_response() -> MsgSignatureResponse {
    MsgSignatureResponse {
        signature: sample_signature(),
    }
}

fn sample_tx_input_with_additional_info() -> TxInputWithAdditionalInfo {
    TxInputWithAdditionalInfo::Account(sample_account_out_point())
}

fn sample_additional_order_info() -> AdditionalOrderInfo {
    AdditionalOrderInfo {
        initially_asked: sample_output_value(),
        initially_given: sample_output_value(),
        ask_balance: sample_amount(),
        give_balance: sample_amount(),
    }
}

fn sample_tx_output() -> TxOutput {
    TxOutput::Burn(sample_output_value())
}

fn sample_account_out_point() -> AccountOutPoint {
    AccountOutPoint {
        nonce: AccountNonce(123),
        spending: sample_account_spending(),
    }
}

fn sample_account_spending() -> AccountSpending {
    AccountSpending::DelegationBalance(sample_delegation_id(), sample_amount())
}

fn sample_account_command() -> AccountCommand {
    AccountCommand::UnmintTokens(sample_token_id())
}

fn sample_order_account_command() -> OrderAccountCommand {
    OrderAccountCommand::FreezeOrder(sample_order_id())
}

fn sample_utxo_out_point() -> UtxoOutPoint {
    UtxoOutPoint::new(sample_out_point_source_id(), 123)
}

fn sample_out_point_source_id() -> OutPointSourceId {
    OutPointSourceId::Transaction(sample_transaction_id())
}

fn sample_stake_pool_data() -> StakePoolData {
    StakePoolData {
        pledge: sample_amount(),
        staker: sample_destination(),
        vrf_public_key: sample_vrf_public_key(),
        decommission_key: sample_destination(),
        margin_ratio_per_thousand: PerThousand(123),
        cost_per_block: sample_amount(),
    }
}

fn sample_order_data() -> OrderData {
    OrderData {
        conclude_key: sample_destination(),
        ask: sample_output_value(),
        give: sample_output_value(),
    }
}

fn sample_hashed_timelock_contract() -> HashedTimelockContract {
    HashedTimelockContract {
        secret_hash: [0x0B; 20].into(),
        spend_key: sample_destination(),
        refund_timelock: sample_output_time_lock(),
        refund_key: sample_destination(),
    }
}

fn sample_token_issuance_v1(total_supply: TokenTotalSupply) -> TokenIssuanceV1 {
    TokenIssuanceV1 {
        token_ticker: vec![1, 2, 3],
        number_of_decimals: 8,
        metadata_uri: vec![4, 5, 6],
        total_supply,
        authority: sample_destination(),
        is_freezable: IsTokenFreezable::No,
    }
}

fn sample_nft_issuance_v0(creator: Option<PublicKey>) -> NftIssuanceV0 {
    NftIssuanceV0 {
        creator,
        name: vec![1, 2, 3],
        description: vec![4, 5, 6],
        ticker: vec![7, 8, 9],
        icon_uri: vec![10, 11, 12],
        additional_metadata_uri: vec![13, 14, 15],
        media_uri: vec![16, 17, 18],
        media_hash: vec![19, 20, 21],
    }
}

fn sample_destination() -> Destination {
    Destination::AnyoneCanSpend
}

fn sample_output_value() -> OutputValue {
    OutputValue::Coin(sample_amount())
}

fn sample_output_time_lock() -> OutputTimeLock {
    OutputTimeLock::ForBlockCount(BlocksCount(123))
}

fn sample_public_key() -> PublicKey {
    PublicKey::Secp256k1Schnorr(sample_secp256k1_public_key())
}

fn sample_vrf_public_key() -> VrfPublicKey {
    VrfPublicKey::Schnorrkel(sample_schnorrkel_public_key())
}

fn sample_input_address_path() -> InputAddressPath {
    InputAddressPath {
        path: sample_bip32_path(),
        multisig_idx: Some(123),
    }
}

fn sample_bip32_path() -> Bip32Path {
    Bip32Path(vec![1, 2, 3])
}

fn sample_amount() -> Amount {
    Amount::from_atoms(123)
}

fn sample_order_id() -> OrderId {
    OrderId::new(sample_h256(0x01))
}

fn sample_token_id() -> TokenId {
    TokenId::new(sample_h256(0x02))
}

fn sample_delegation_id() -> DelegationId {
    DelegationId::new(sample_h256(0x03))
}

fn sample_pool_id() -> PoolId {
    PoolId::new(sample_h256(0x04))
}

fn sample_transaction_id() -> TransactionId {
    TransactionId::new(sample_h256(0x05))
}

fn sample_gen_block_id() -> GenBlockId {
    GenBlockId::new(sample_h256(0x06))
}

fn sample_script_id() -> ScriptId {
    ScriptId::new(sample_h256(0x07))
}

fn sample_h256(byte: u8) -> H256 {
    [byte; 32].into()
}

fn sample_public_key_hash() -> PublicKeyHash {
    [0x08; 20].into()
}

fn sample_secp256k1_public_key() -> Secp256k1PublicKey {
    Secp256k1PublicKey([0x09; 33])
}

fn sample_schnorrkel_public_key() -> SchnorrkelPublicKey {
    SchnorrkelPublicKey([0x0A; 32])
}

fn sample_signature() -> Signature {
    Signature([0xAB; 64])
}
