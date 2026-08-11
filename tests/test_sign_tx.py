from application_client import MAINNET
from application_client.mintlayer_command_sender import (
    Errors,
    MintlayerCommandSender,
    ReviewUntil,
    send_output_expect_error,
    fetch_public_key_as_pk_destination,
    fetch_public_key_as_pkh_destination,
    sign_tx_review,
    ReviewTransaction,
)
from application_client.mintlayer_utils import (
    KeyPurpose,
    Transaction,
    make_path,
    sign_tx_next_req_obj,
)

# TODO: implement missing tests:
# * CreateOrder, DataDeposit, Burn outputs;
# * pool decommissioning;
# * LockTokenSupply, FreezeOrder commands;
# * maybe something else.
# See https://github.com/mintlayer/mintlayer-ledger-app/issues/19.


# Test a simple transfer without a change output.
def test_sign_tx_transfer_no_change(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)

    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 10},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    inp = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([0] * 32)},
                    "index": 1,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    output = {
        "output": {
            "Transfer": [
                {"Coin": 10},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
            ],
        },
        "change_path": None,
    }
    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[inp],
        input_commitments=[inp_commitment],
        outputs=[output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\stransfer",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


# Test a simple transfer with a change output whose change_path is None.
def test_sign_tx_transfer_change_output_without_change_path(
    backend, scenario_navigator, device, navigator
):
    _sign_tx_transfer_with_change_output(
        backend,
        scenario_navigator,
        device,
        navigator,
        lambda client, path, amount: _make_transfer_output(
            amount,
            fetch_public_key_as_pk_destination(client, MAINNET, path),
            None,
        ),
    )


# Test a simple transfer with a change output whose change_path is not None and the destination
# is the PublicKey one.
def test_sign_tx_transfer_change_output_public_key(
    backend, scenario_navigator, device, navigator
):
    _sign_tx_transfer_with_change_output(
        backend,
        scenario_navigator,
        device,
        navigator,
        lambda client, path, amount: _make_transfer_output(
            amount,
            fetch_public_key_as_pk_destination(client, MAINNET, path),
            path,
        ),
    )


# Test a simple transfer with a change output whose change_path is not None and the destination
# is the PublicKeyHash one.
def test_sign_tx_transfer_change_output_public_key_hash(
    backend, scenario_navigator, device, navigator
):
    _sign_tx_transfer_with_change_output(
        backend,
        scenario_navigator,
        device,
        navigator,
        lambda client, path, amount: _make_transfer_output(
            amount,
            fetch_public_key_as_pkh_destination(client, MAINNET, path),
            path,
        ),
    )


def _sign_tx_transfer_with_change_output(
    backend,
    scenario_navigator,
    device,
    navigator,
    make_change_output,
):
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    bip44_change_path = make_path(0, KeyPurpose.Change, 0)

    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 10},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    inp = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([0] * 32)},
                    "index": 1,
                },
                additional_info,
            ]
        },
    }
    inp_commitment = {"commitment": additional_info}

    recipient_output = _make_transfer_output(
        9,
        {
            "PublicKey": {
                "key": {
                    "Secp256k1Schnorr": {
                        "pubkey_data": bytes([0] * 33),
                    }
                }
            }
        },
        None,
    )
    change_output = make_change_output(client, bip44_change_path, 1)

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[inp],
        input_commitments=[inp_commitment],
        outputs=[recipient_output, change_output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\stransfer",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


# Test a simple transfer with a change output whose change_path is not None and is invalid (has
# an extra element).
def test_sign_tx_transfer_change_output_invalid_change_path(
    backend, scenario_navigator, device, navigator
):
    client = MintlayerCommandSender(backend)
    change_path = make_path(0, KeyPurpose.Change, 0)

    _assert_sign_tx_transfer_change_path_fails(
        client,
        scenario_navigator,
        device,
        navigator,
        lambda amount: _make_transfer_output(
            amount,
            fetch_public_key_as_pk_destination(client, MAINNET, change_path),
            change_path + [0],
        ),
        Errors.SW_INVALID_PATH,
    )


# Test a simple transfer with a change output whose change_path is not None, is valid, but
# doesn't match the actual destination.
def test_sign_tx_transfer_change_output_mismatched_destination(
    backend, scenario_navigator, device, navigator
):
    client = MintlayerCommandSender(backend)
    destination_path = make_path(0, KeyPurpose.Change, 0)
    change_path = make_path(0, KeyPurpose.Change, 1)

    _assert_sign_tx_transfer_change_path_fails(
        client,
        scenario_navigator,
        device,
        navigator,
        lambda amount: _make_transfer_output(
            amount,
            fetch_public_key_as_pk_destination(client, MAINNET, destination_path),
            change_path,
        ),
        Errors.SW_MISMATCHED_CHANGE_OUTPUT_DESTINATION,
    )


# Test a simple transfer with a change output whose change_path is not None, is invalid, matches
# the actual destination, but is a "receive" path, not a "change" one.
def test_sign_tx_transfer_change_output_receive_path(
    backend, scenario_navigator, device, navigator
):
    client = MintlayerCommandSender(backend)
    receive_path = make_path(0, KeyPurpose.Receive, 1)

    _assert_sign_tx_transfer_change_path_fails(
        client,
        scenario_navigator,
        device,
        navigator,
        lambda amount: _make_transfer_output(
            amount,
            fetch_public_key_as_pk_destination(client, MAINNET, receive_path),
            receive_path,
        ),
        Errors.SW_INVALID_PATH,
    )


def _assert_sign_tx_transfer_change_path_fails(
    client,
    scenario_navigator,
    device,
    navigator,
    make_change_output,
    expected_status,
):
    bip44_path = make_path(0, KeyPurpose.Receive, 0)

    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 10},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    inp = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([0] * 32)},
                    "index": 1,
                },
                additional_info,
            ]
        },
    }
    inp_commitment = {"commitment": additional_info}
    output = make_change_output(1)

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[inp],
        input_commitments=[inp_commitment],
        outputs=[output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\stransfer",
    )
    sign_tx_review(
        client,
        device,
        navigator,
        scenario_navigator,
        review_tx,
        ReviewUntil.Outputs,
    )
    send_output_expect_error(client, transaction.outputs[0], expected_status)


def test_sign_tx_lock_then_transfer(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)

    inp = {
        "addresses": [
            {"path": make_path(0, KeyPurpose.Receive, 0), "multisig_idx": None}
        ],
        "input": {
            "Account": {
                "nonce": 1,
                "spending": {"DelegationBalance": [[0] * 32, 11]},
            }
        },
    }

    inp_commitment = {"commitment": {"None": None}}

    output = {
        "output": {
            "LockThenTransfer": [
                {"Coin": 10},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
                {"UntilHeight": 10},
            ],
        },
        "change_path": None,
    }

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[inp],
        input_commitments=[inp_commitment],
        outputs=[output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=True,
        review_custom_screen_text=r"Sign\sdelegation",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_create_delegation(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)

    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 10},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    inp = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([0] * 32)},
                    "index": 1,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    output = {
        "output": {
            "CreateDelegationId": [
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
                [0] * 32,
            ],
        },
        "change_path": None,
    }

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[inp],
        input_commitments=[inp_commitment],
        outputs=[output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\screate",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_delegate_staking(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)

    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 10},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    inp = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([0] * 32)},
                    "index": 1,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    output = {
        "output": {
            "DelegateStaking": [5, [0] * 32],
        },
        "change_path": None,
    }

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[inp],
        input_commitments=[inp_commitment],
        outputs=[output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\sdelegate",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_create_stake_pool(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)

    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 40001},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    inp = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([0] * 32)},
                    "index": 1,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    output = {
        "output": {
            "CreateStakePool": [
                [0] * 32,
                {
                    "pledge": 40000,
                    "staker": {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                    "vrf_public_key": {"key": {"Schnorrkel": {"key": bytes([0] * 32)}}},
                    "decommission_key": {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                    "margin_ratio_per_thousand": 10,
                    "cost_per_block": 5,
                },
            ],
        },
        "change_path": None,
    }
    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[inp],
        input_commitments=[inp_commitment],
        outputs=[output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\screate\sstake",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_issue_fungible_token(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 10},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    inp = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([0] * 32)},
                    "index": 1,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    output = {
        "output": {
            "IssueFungibleToken": {
                "V1": {
                    "token_ticker": b"MYTKN",
                    "number_of_decimals": 8,
                    "metadata_uri": b"https://my.token.uri",
                    "total_supply": {"Fixed": 1000000000},
                    "authority": {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                    "is_freezable": {"Yes": None},
                }
            },
        },
        "change_path": None,
    }

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[inp],
        input_commitments=[inp_commitment],
        outputs=[output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\screate\stoken",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_issue_nft(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 2000},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    inp = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([0] * 32)},
                    "index": 1,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    # This is the new output for issuing an NFT.
    # The structure is (TokenId, NftIssuance, Destination)
    output = {
        "output": {
            "IssueNft": [
                bytes([0] * 32),
                {
                    "V0": {
                        "metadata": {
                            "creator": {
                                "public_key": {
                                    "key": {
                                        "Secp256k1Schnorr": {
                                            "pubkey_data": bytes([0] * 33)
                                        }
                                    }
                                }
                            },
                            "name": b"MyAwesomeNFT",
                            "description": b"FirstNFT",
                            "ticker": b"MNFT1",
                            "icon_uri": b"https://my.nft/icon.png",
                            "additional_metadata_uri": b"https://my.nft/meta.json",
                            "media_uri": b"https://my.nft/media.jpg",
                            "media_hash": bytes([0] * 32),
                        }
                    }
                },
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
            ],
        },
        "change_path": None,
    }

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[inp],
        input_commitments=[inp_commitment],
        outputs=[output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\screate\sNFT",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_mint_tokens(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input.
    2. An AccountCommand input to mint new tokens.
    And one output to transfer the newly minted tokens.
    """
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)

    # The utxo (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    utxo_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([1] * 32)},
                    "index": 0,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "AccountCommand": [
                account_nonce,
                {
                    "MintTokens": [
                        token_id,
                        1000,  # Amount to mint
                    ]
                },
            ]
        },
    }

    acc_inp_commitment = {"commitment": {"None": None}}

    mint_output = {
        "output": {
            "Transfer": [
                {"TokenV1": [token_id, 1000]},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        },
        "change_path": None,
    }

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, acc_inp_commitment],
        outputs=[mint_output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=True,
        review_custom_screen_text=r"Sign\smint\stokens",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_unmint_tokens(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay tx fees.
    2. An AccountCommand input to unmint new tokens.
    3. A standard UTXO input with tokens to unmint
    And one output to transfer the newly minted tokens.
    """
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    bip44_change_path = make_path(0, KeyPurpose.Change, 0)

    # The additional data (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    additional_info2 = {
        "Utxo": {
            "Transfer": [
                {"TokenV1": [bytes([0] * 32), 1000]},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    utxo_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([1] * 32)},
                    "index": 0,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    utxo_input2 = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([1] * 32)},
                    "index": 2,
                },
                additional_info2,
            ]
        },
    }

    inp_commitment2 = {"commitment": additional_info2}

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "AccountCommand": [
                account_nonce,
                {
                    "UnmintTokens": token_id,
                },
            ]
        },
    }

    acc_inp_commitment = {"commitment": {"None": None}}

    change_output = _make_transfer_output(
        99,
        fetch_public_key_as_pk_destination(client, MAINNET, bip44_change_path),
        bip44_change_path,
    )

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[utxo_input, account_input, utxo_input2],
        input_commitments=[inp_commitment, acc_inp_commitment, inp_commitment2],
        outputs=[change_output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=True,
        review_custom_screen_text=r"Sign\sunmint\stokens",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_freeze_tokens(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An AccountCommand input to freeze the tokens.
    And one output to transfer the change coins.
    """
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    bip44_change_path = make_path(0, KeyPurpose.Change, 0)

    # The additional info (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }

    utxo_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([1] * 32)},
                    "index": 0,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "AccountCommand": [
                account_nonce,
                {"FreezeToken": [token_id, {"No": None}]},
            ]
        },
    }

    acc_inp_commitment = {"commitment": {"None": None}}

    change_output = _make_transfer_output(
        99,
        fetch_public_key_as_pk_destination(client, MAINNET, bip44_change_path),
        bip44_change_path,
    )

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, acc_inp_commitment],
        outputs=[change_output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=True,
        review_custom_screen_text=r"Sign\sfreeze\stokens",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_unfreeze_tokens(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An AccountCommand input to unfreeze the tokens.
    And one output to transfer the change coins.
    """
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    bip44_change_path = make_path(0, KeyPurpose.Change, 0)

    # The additional data (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    utxo_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([1] * 32)},
                    "index": 0,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "AccountCommand": [
                account_nonce,
                {
                    "UnfreezeToken": token_id,
                },
            ]
        },
    }

    acc_inp_commitment = {"commitment": {"None": None}}

    change_output = _make_transfer_output(
        99,
        fetch_public_key_as_pk_destination(client, MAINNET, bip44_change_path),
        bip44_change_path,
    )

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, acc_inp_commitment],
        outputs=[change_output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=True,
        review_custom_screen_text=r"Sign\sunfreeze",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_change_token_authority(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An AccountCommand input to change the token's authority.
    And one output to transfer the change coins.
    """
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    bip44_change_path = make_path(0, KeyPurpose.Change, 0)

    # The additional data (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    utxo_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([1] * 32)},
                    "index": 0,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "AccountCommand": [
                account_nonce,
                {
                    "ChangeTokenAuthority": [
                        token_id,
                        {
                            "PublicKey": {
                                "key": {
                                    "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                                }
                            }
                        },
                    ]
                },
            ]
        },
    }

    acc_inp_commitment = {"commitment": {"None": None}}

    change_output = _make_transfer_output(
        99,
        fetch_public_key_as_pk_destination(client, MAINNET, bip44_change_path),
        bip44_change_path,
    )

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, acc_inp_commitment],
        outputs=[change_output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=True,
        review_custom_screen_text=r"Sign\schange\stoken",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_change_token_metadata_uri(
    backend, scenario_navigator, device, navigator
):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An AccountCommand input to change the token's metadata uri.
    And one output to transfer the change coins.
    """
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    bip44_change_path = make_path(0, KeyPurpose.Change, 0)

    # The additional info (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    utxo_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([1] * 32)},
                    "index": 0,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "AccountCommand": [
                account_nonce,
                {
                    "ChangeTokenMetadataUri": [
                        token_id,
                        "uri.com".encode(),
                    ]
                },
            ]
        },
    }

    acc_inp_commitment = {"commitment": {"None": None}}

    change_output = _make_transfer_output(
        99,
        fetch_public_key_as_pk_destination(client, MAINNET, bip44_change_path),
        bip44_change_path,
    )

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, acc_inp_commitment],
        outputs=[change_output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=True,
        review_custom_screen_text=r"Sign\schange\stoken",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_order_fill(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with 3 inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An account command input to fill an order.
    3. A standard UTXO input to be used for the fill.
    And one output to transfer the change coins.
    """
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    bip44_change_path = make_path(0, KeyPurpose.Change, 0)

    # The additional info (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    utxo_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([1] * 32)},
                    "index": 0,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    fill_amount = 10
    fill_ask = 100
    fill_give = 1000

    additional_order_info = {
        "initially_asked": {"Coin": fill_ask},
        "initially_given": {"TokenV1": [bytes([0] * 32), fill_give]},
        "ask_balance": 0,
        "give_balance": 0,
    }
    order_id = bytes([0] * 32)
    account_input = {
        "addresses": [],  # FillOrder input must not be signed
        "input": {
            "OrderAccountCommand": [
                {
                    "FillOrder": [
                        order_id,
                        fill_amount,
                    ]
                },
                additional_order_info,
            ]
        },
    }

    fill_order_inp_commitment = {
        "commitment": {
            "FillOrderAccountCommand": [
                additional_order_info["initially_asked"],
                additional_order_info["initially_given"],
            ]
        },
    }

    change_output = _make_transfer_output(
        100 - 1 - fill_amount,
        fetch_public_key_as_pk_destination(client, MAINNET, bip44_change_path),
        bip44_change_path,
    )

    fill_output = {
        "output": {
            "Transfer": [
                {
                    "TokenV1": [
                        bytes([0] * 32),
                        fill_amount * fill_give // fill_ask,
                    ]
                },
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        },
        "change_path": None,
    }

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, fill_order_inp_commitment],
        outputs=[change_output, fill_output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=True,
        review_custom_screen_text=r"Sign\sfill\sorder",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_order_conclude(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An OrderAccountCommand input to conclude an order.
    And one output to transfer the change coins plus the filled amount (where the latter
    is initial_ask minus ask_balance) and another output for the give balance.
    """
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    bip44_change_path = make_path(0, KeyPurpose.Change, 0)

    # The additional_data (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    utxo_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([1] * 32)},
                    "index": 0,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    initial_ask = 100
    initial_give = 1000
    ask_balance = 90
    give_balance = 900

    token_id = bytes([1] * 32)
    additional_order_info = {
        "initially_asked": {"Coin": initial_ask},
        "initially_given": {"TokenV1": [token_id, initial_give]},
        "ask_balance": ask_balance,
        "give_balance": give_balance,
    }
    order_id = bytes([1] * 32)
    account_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "OrderAccountCommand": [
                {
                    "ConcludeOrder": order_id,
                },
                additional_order_info,
            ]
        },
    }

    conclude_order_inp_commitment = {
        "commitment": {
            "ConcludeOrderAccountCommand": [
                additional_order_info["initially_asked"],
                additional_order_info["initially_given"],
                additional_order_info["ask_balance"],
                additional_order_info["give_balance"],
            ]
        },
    }

    change_output = _make_transfer_output(
        100 - 1 + initial_ask - ask_balance,
        fetch_public_key_as_pk_destination(client, MAINNET, bip44_change_path),
        bip44_change_path,
    )

    conclude_output = {
        "output": {
            "Transfer": [
                {"TokenV1": [token_id, give_balance]},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        },
        "change_path": None,
    }

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, conclude_order_inp_commitment],
        outputs=[change_output, conclude_output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=True,
        review_custom_screen_text=r"Sign\sconclude\sorder",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_htlc(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with one input:
    1. The standard UTXO input to pay for tx fees.
    And one output to transfer the change coins and the HTLC output.
    """
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    bip44_change_path = make_path(0, KeyPurpose.Change, 0)

    # The additional info (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    utxo_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([1] * 32)},
                    "index": 0,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    change_output = _make_transfer_output(
        89,
        fetch_public_key_as_pk_destination(client, MAINNET, bip44_change_path),
        bip44_change_path,
    )

    htlc_output = {
        "output": {
            "Htlc": [
                {"Coin": 10},
                {
                    "secret_hash": [0] * 20,
                    "spend_key": {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                    "refund_timelock": {"UntilHeight": 100},
                    "refund_key": {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([3] * 33)}
                            }
                        }
                    },
                },
            ],
        },
        "change_path": None,
    }

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[utxo_input],
        input_commitments=[inp_commitment],
        outputs=[htlc_output, change_output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\screate\sHTLC",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_without_outputs(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An AccountCommand input to freeze the tokens.
    And no outputs.
    """
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)

    # The additional info (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }

    utxo_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([1] * 32)},
                    "index": 0,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "AccountCommand": [
                account_nonce,
                {"FreezeToken": [token_id, {"No": None}]},
            ]
        },
    }

    acc_inp_commitment = {"commitment": {"None": None}}

    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, acc_inp_commitment],
        outputs=[],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=True,
        review_custom_screen_text=r"Sign\sfreeze\stokens",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


# Sign a tx with an output large enough to require chunking
def test_sign_tx_with_large_output(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 2000},
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    inp = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([0] * 32)},
                    "index": 1,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    # Make the output big
    metadata_uri = b"abcef" * 100

    # This is an output for issuing an NFT.
    # The structure is (TokenId, NftIssuance, Destination)
    output = {
        "output": {
            "IssueNft": [
                bytes([0] * 32),
                {
                    "V0": {
                        "metadata": {
                            "creator": {
                                "public_key": {
                                    "key": {
                                        "Secp256k1Schnorr": {
                                            "pubkey_data": bytes([0] * 33)
                                        }
                                    }
                                }
                            },
                            "name": b"MyAwesomeNFT",
                            "description": b"FirstNFT",
                            "ticker": b"MNFT1",
                            "icon_uri": b"https://my.nft/icon.png",
                            "additional_metadata_uri": metadata_uri,
                            "media_uri": b"https://my.nft/media.jpg",
                            "media_hash": bytes([0] * 32),
                        }
                    }
                },
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
            ],
        },
        "change_path": None,
    }

    # Sanity check
    assert len(sign_tx_next_req_obj.encode({"ProcessOutput": output}).data) > 500

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[inp],
        input_commitments=[inp_commitment],
        outputs=[output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\screate\sNFT",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


# Sign a tx with an input and input commitment large enough to require chunking
def test_sign_tx_with_large_input_and_commitment(
    backend, scenario_navigator, device, navigator
):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    bip44_path = make_path(0, KeyPurpose.Receive, 0)

    # Make the additional info big
    metadata_uri = b"abcef" * 100

    # The utxo is for issuing an NFT.
    # The structure is (TokenId, NftIssuance, Destination)
    token_id = bytes([11] * 32)
    additional_info = {
        "Utxo": {
            "IssueNft": [
                token_id,
                {
                    "V0": {
                        "metadata": {
                            "creator": {
                                "public_key": {
                                    "key": {
                                        "Secp256k1Schnorr": {
                                            "pubkey_data": bytes([0] * 33)
                                        }
                                    }
                                }
                            },
                            "name": b"MyAwesomeNFT",
                            "description": b"FirstNFT",
                            "ticker": b"MNFT1",
                            "icon_uri": b"https://my.nft/icon.png",
                            "additional_metadata_uri": metadata_uri,
                            "media_uri": b"https://my.nft/media.jpg",
                            "media_hash": bytes([0] * 32),
                        }
                    }
                },
                fetch_public_key_as_pk_destination(client, MAINNET, bip44_path),
            ],
        }
    }
    inp = {
        "addresses": [{"path": bip44_path, "multisig_idx": None}],
        "input": {
            "Utxo": [
                {
                    "id": {"Transaction": bytes([0] * 32)},
                    "index": 1,
                },
                additional_info,
            ]
        },
    }

    inp_commitment = {"commitment": additional_info}

    output = {
        "output": {
            "Transfer": [
                {"TokenV1": [token_id, 1]},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        },
        "change_path": None,
    }

    # Sanity checks
    assert len(sign_tx_next_req_obj.encode({"ProcessInput": inp}).data) > 500
    assert (
        len(
            sign_tx_next_req_obj.encode({"ProcessInputCommitment": inp_commitment}).data
        )
        > 500
    )

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin_type=MAINNET,
        inputs=[inp],
        input_commitments=[inp_commitment],
        outputs=[output],
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\stransfer",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def _make_transfer_output(amount, destination, change_path):
    return {
        "output": {
            "Transfer": [
                {"Coin": amount},
                destination,
            ],
        },
        "change_path": change_path,
    }
