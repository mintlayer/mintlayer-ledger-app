import pytest
from ragger.navigator import NavIns, NavInsID

from application_client import MAINNET
from application_client.mintlayer_command_sender import (
    MintlayerCommandSender,
    sign_tx_review,
    ReviewTransaction,
)
from application_client.mintlayer_response_unpacker import (
    unpack_get_public_key_response,
)
from application_client.mintlayer_utils import Transaction, sign_tx_next_req_obj


def test_sign_tx_transfer(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    # The path used for this entire test
    path: str = "m/44'/19788'/0'/0/0"

    # First we need to get the public key of the device in order to build the transaction
    rapdu = client.get_public_key(coin=MAINNET, path=path)
    _, public_key, _, _ = unpack_get_public_key_response(rapdu.data)

    print("pk", len(public_key))

    h = 1 << 31
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 10},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
            ],
        }
    }
    inp = {
        "ProcessInput": {
            "addresses": [
                {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
            ],
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
    }

    inp_commitment = {"ProcessInputCommitment": {"commitment": additional_info}}

    output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"Coin": 10},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }
    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\stransfer",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)


def test_sign_tx_lock_then_transfer(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    # The path used for this entire test
    path: str = "m/44'/19788'/0'/0/0"

    h = 1 << 31
    inp = {
        "ProcessInput": {
            "addresses": [
                {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
            ],
            "input": {
                "Account": {
                    "nonce": 1,
                    "account": {"Delegation": [[0] * 32, 11]},
                }
            },
        }
    }

    inp_commitment = {"ProcessInputCommitment": {"commitment": {"None": None}}}

    output = {
        "ProcessOutput": {
            "output": {
                "LockThenTransfer": [
                    {"Coin": 10},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                    {"UntilHeight": 10},
                ],
            }
        }
    }

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
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
    h = 1 << 31

    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 10},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
            ],
        }
    }
    inp = {
        "ProcessInput": {
            "addresses": [
                {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
            ],
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
    }

    inp_commitment = {"ProcessInputCommitment": {"commitment": additional_info}}

    output = {
        "ProcessOutput": {
            "output": {
                "CreateDelegationId": [
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                    [0] * 32,
                ],
            }
        }
    }

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
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
    h = 1 << 31

    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 10},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
            ],
        }
    }
    inp = {
        "ProcessInput": {
            "addresses": [
                {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
            ],
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
    }

    inp_commitment = {"ProcessInputCommitment": {"commitment": additional_info}}

    output = {
        "ProcessOutput": {
            "output": {
                "DelegateStaking": [5, [0] * 32],
            }
        }
    }

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
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
    h = 1 << 31

    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 40001},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
            ],
        }
    }
    inp = {
        "ProcessInput": {
            "addresses": [
                {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
            ],
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
    }

    inp_commitment = {"ProcessInputCommitment": {"commitment": additional_info}}

    output = {
        "ProcessOutput": {
            "output": {
                "CreateStakePool": [
                    [0] * 32,
                    {
                        "value": 40000,
                        "staker": {
                            "PublicKey": {
                                "key": {
                                    "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                                }
                            }
                        },
                        "vrf_public_key": {
                            "key": {"Schnorrkel": {"key": bytes([0] * 32)}}
                        },
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
            }
        }
    }
    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
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
    h = 1 << 31
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 10},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
            ],
        }
    }
    inp = {
        "ProcessInput": {
            "addresses": [
                {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
            ],
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
    }

    inp_commitment = {"ProcessInputCommitment": {"commitment": additional_info}}

    output = {
        "ProcessOutput": {
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
            }
        }
    }

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
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
    h = 1 << 31
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 2000},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
            ],
        }
    }
    inp = {
        "ProcessInput": {
            "addresses": [
                {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
            ],
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
    }

    inp_commitment = {"ProcessInputCommitment": {"commitment": additional_info}}

    # This is the new output for issuing an NFT.
    # The structure is (TokenId, NftIssuance, Destination)
    output = {
        "ProcessOutput": {
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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
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
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    # The utxo (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        }
    }
    utxo_input = {
        "ProcessInput": {
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
    }

    inp_commitment = {"ProcessInputCommitment": {"commitment": additional_info}}

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "ProcessInput": {
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
    }

    acc_inp_commitment = {"ProcessInputCommitment": {"commitment": {"None": None}}}

    mint_output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"TokenV1": [token_id, 1000]},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    transaction = Transaction(
        coin=MAINNET,
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
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    # The additional data (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        }
    }
    additional_info2 = {
        "Utxo": {
            "Transfer": [
                {"TokenV1": [bytes([0] * 32), 1000]},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        }
    }
    utxo_input = {
        "ProcessInput": {
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
    }

    inp_commitment = {
        "ProcessInputCommitment": {"commitment": additional_info},
    }

    utxo_input2 = {
        "ProcessInput": {
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
    }

    inp_commitment2 = {"ProcessInputCommitment": {"commitment": additional_info2}}

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "ProcessInput": {
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
    }

    acc_inp_commitment = {"ProcessInputCommitment": {"commitment": {"None": None}}}

    change_output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"Coin": 99},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    transaction = Transaction(
        coin=MAINNET,
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
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    # The additional info (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        }
    }

    utxo_input = {
        "ProcessInput": {
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
    }

    inp_commitment = {
        "ProcessInputCommitment": {"commitment": additional_info},
    }

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "ProcessInput": {
            "addresses": [{"path": bip44_path, "multisig_idx": None}],
            "input": {
                "AccountCommand": [
                    account_nonce,
                    {"FreezeToken": [token_id, {"No": None}]},
                ]
            },
        }
    }

    acc_inp_commitment = {"ProcessInputCommitment": {"commitment": {"None": None}}}

    change_output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"Coin": 99},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    transaction = Transaction(
        coin=MAINNET,
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
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    # The additional data (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        }
    }
    utxo_input = {
        "ProcessInput": {
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
    }

    inp_commitment = {
        "ProcessInputCommitment": {"commitment": additional_info},
    }

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "ProcessInput": {
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
    }

    acc_inp_commitment = {"ProcessInputCommitment": {"commitment": {"None": None}}}

    change_output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"Coin": 99},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    transaction = Transaction(
        coin=MAINNET,
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
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    # The additional data (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        }
    }
    utxo_input = {
        "ProcessInput": {
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
    }

    inp_commitment = {
        "ProcessInputCommitment": {"commitment": additional_info},
    }

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "ProcessInput": {
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
                                        "Secp256k1Schnorr": {
                                            "pubkey_data": bytes([2] * 33)
                                        }
                                    }
                                }
                            },
                        ]
                    },
                ]
            },
        }
    }

    acc_inp_commitment = {"ProcessInputCommitment": {"commitment": {"None": None}}}

    change_output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"Coin": 99},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    transaction = Transaction(
        coin=MAINNET,
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
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    # The additional info (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        }
    }
    utxo_input = {
        "ProcessInput": {
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
    }

    inp_commitment = {
        "ProcessInputCommitment": {"commitment": additional_info},
    }

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "ProcessInput": {
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
    }

    acc_inp_commitment = {"ProcessInputCommitment": {"commitment": {"None": None}}}

    change_output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"Coin": 99},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    transaction = Transaction(
        coin=MAINNET,
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
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An AccountCommand input to fill an order.
    3. A standard UTXO input to be used for the fill.
    And one output to transfer the change coins.
    """
    client = MintlayerCommandSender(backend)
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    # The additionl info (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        }
    }
    utxo_input = {
        "ProcessInput": {
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
    }

    inp_commitment = {
        "ProcessInputCommitment": {"commitment": additional_info},
    }

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
        "ProcessInput": {
            "addresses": [], # FillOrder input must not be signed
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
    }

    fill_order_inp_commitment = {
        "ProcessInputCommitment": {
            "commitment": {
                "FillOrderAccountCommand": [
                    additional_order_info["initially_asked"],
                    additional_order_info["initially_given"],
                ]
            },
        }
    }

    change_output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"Coin": 100 - 1 - fill_amount},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    fill_output = {
        "ProcessOutput": {
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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    transaction = Transaction(
        coin=MAINNET,
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
    And one output to transfer the change coins + ask balance and another output for the give balance.
    """
    client = MintlayerCommandSender(backend)
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    # The additional_data (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        }
    }
    utxo_input = {
        "ProcessInput": {
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
    }

    inp_commitment = {
        "ProcessInputCommitment": {"commitment": additional_info},
    }

    initial_ask = 100
    initial_give = 1000
    ask_balance = 10
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
        "ProcessInput": {
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
    }

    conclude_order_inp_commitment = {
        "ProcessInputCommitment": {
            "commitment": {
                "ConcludeOrderAccountCommand": [
                    additional_order_info["initially_asked"],
                    additional_order_info["initially_given"],
                    additional_order_info["ask_balance"],
                    additional_order_info["give_balance"],
                ]
            },
        }
    }

    change_output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"Coin": 100 - 1 + ask_balance},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    conclude_output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"TokenV1": [token_id, give_balance]},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    transaction = Transaction(
        coin=MAINNET,
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
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    # The additional info (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        }
    }
    utxo_input = {
        "ProcessInput": {
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
    }

    inp_commitment = {
        "ProcessInputCommitment": {"commitment": additional_info},
    }

    change_output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"Coin": 89},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    htlc_output = {
        "ProcessOutput": {
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
            }
        }
    }

    transaction = Transaction(
        coin=MAINNET,
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
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    # The additional info (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 100},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                    }
                },
            ],
        }
    }

    utxo_input = {
        "ProcessInput": {
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
    }

    inp_commitment = {
        "ProcessInputCommitment": {"commitment": additional_info},
    }

    account_nonce = 1
    token_id = bytes([0] * 32)
    account_input = {
        "ProcessInput": {
            "addresses": [{"path": bip44_path, "multisig_idx": None}],
            "input": {
                "AccountCommand": [
                    account_nonce,
                    {"FreezeToken": [token_id, {"No": None}]},
                ]
            },
        }
    }

    acc_inp_commitment = {"ProcessInputCommitment": {"commitment": {"None": None}}}

    transaction = Transaction(
        coin=MAINNET,
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
    h = 1 << 31
    additional_info = {
        "Utxo": {
            "Transfer": [
                {"Coin": 2000},
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
            ],
        }
    }
    inp = {
        "ProcessInput": {
            "addresses": [
                {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
            ],
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
    }

    inp_commitment = {"ProcessInputCommitment": {"commitment": additional_info}}

    # Make the output big
    metadata_uri = b"abcef" * 100

    # This is an output for issuing an NFT.
    # The structure is (TokenId, NftIssuance, Destination)
    output = {
        "ProcessOutput": {
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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    # Sanity check
    assert len(sign_tx_next_req_obj.encode(output).data) > 500

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
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
    h = 1 << 31

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
                {
                    "PublicKey": {
                        "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                    }
                },
            ],
        }
    }
    inp = {
        "ProcessInput": {
            "addresses": [
                {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
            ],
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
    }

    inp_commitment = {"ProcessInputCommitment": {"commitment": additional_info}}

    output = {
        "ProcessOutput": {
            "output": {
                "Transfer": [
                    {"TokenV1": [token_id, 1]},
                    {
                        "PublicKey": {
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    }

    # Sanity checks
    assert len(sign_tx_next_req_obj.encode(inp).data) > 500
    assert len(sign_tx_next_req_obj.encode(inp_commitment).data) > 500

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
    )

    review_tx = ReviewTransaction(
        transaction=transaction,
        has_command_input=False,
        review_custom_screen_text=r"Sign\stransfer",
    )
    sign_tx_review(client, device, navigator, scenario_navigator, review_tx)
