import pytest
import scalecodec
from ragger.navigator import NavIns, NavInsID

from application_client import MAINNET
from application_client.mintlayer_command_sender import MintlayerCommandSender, sign_tx_review
from application_client.mintlayer_response_unpacker import unpack_get_public_key_response
from application_client.mintlayer_transaction import Transaction

sign_tx_req_obj = scalecodec.base.RuntimeConfiguration().create_scale_object("SignTxReq")

TX_RESPONSE_SIZE = 71


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    inp = sign_tx_req_obj.encode(
        {
            "Input": 
                {
                    "addresses": [
                        {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
                    ],
                    "inp": {
                        "Utxo": [
                            {
                                "id": {"Transaction": "0x{}".format(bytes([0] * 32).hex())},
                                "index": 1,
                            },
                            additional_info,
                        ]
                    }
                }
            }
        ).data
    
    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info
        }
        ).data

    output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
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
        }
    ).data
    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=False, review_custom_screen_text=r"Sign\stransfer")


def test_sign_tx_lock_then_transfer(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    # The path used for this entire test
    path: str = "m/44'/19788'/0'/0/0"

    h = 1 << 31
    inp = sign_tx_req_obj.encode(
        {
            "Input": 
                {
                    "addresses": [
                        {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
                    ],
                    "inp": {
                        "Account": {
                            "nonce": 1,
                            "account": {"Delegation": [[0] * 32, 11]},
                        }
                    },
                }
            }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": {"None": None}
        }
    ).data

    output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "LockThenTransfer": [
                        {"Coin": 10},
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                            }
                        },
                        {"UntilHeight": 10},
                    ],
                }
            }
        }
    ).data

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=True, review_custom_screen_text=r"Sign\swithdrawal")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    inp = sign_tx_req_obj.encode(
        {
            "Input": 
                {
                    "addresses": [
                        {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
                    ],
                    "inp": {
                        "Utxo": [
                            {
                            "id": {"Transaction": "0x{}".format(bytes([0] * 32).hex())},
                            "index": 1,
                            },
                            additional_info,
                        ]
                    },
                }
            }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info
        }
    ).data

    output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "CreateDelegationId": [
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}}
                            }
                        },
                        [0] * 32,
                    ],
                }
            }
        }
    ).data

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=False, review_custom_screen_text=r"Sign\screate")


def test_sign_tx_delegation_staking(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    h = 1 << 31

    additional_info = {
            "Utxo": {
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
    inp = sign_tx_req_obj.encode(
        {
            "Input": 
                {
                    "addresses": [
                        {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
                    ],
                    "inp": {
                        "Utxo": [
                            {
                                "id": {"Transaction": "0x{}".format(bytes([0] * 32).hex())},
                                "index": 1,
                            },
                            additional_info,
                        ]
                    },
                }
            }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info
        }
    ).data

    output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "DelegateStaking": [5, [0] * 32],
                }
            }
        }
    ).data

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=False, review_custom_screen_text=r"Sign\sstake")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    inp = sign_tx_req_obj.encode(
        {
            "Input": 
                {
                    "addresses": [
                        {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
                    ],
                    "inp": {
                        "Utxo": [
                            {
                                "id": {"Transaction": "0x{}".format(bytes([0] * 32).hex())},
                                "index": 1,
                            },
                            additional_info,
                        ]
                    },
                }
            }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info
        }
    ).data

    output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
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
                }
            }
        }
    ).data
    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=False, review_custom_screen_text=r"Sign\screate\sstake")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    inp = sign_tx_req_obj.encode(
        {
            "Input": 
                {
                    "addresses": [
                        {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
                    ],
                    "inp": {
                        "Utxo": [
                            {
                                "id": {"Transaction": "0x{}".format(bytes([0] * 32).hex())},
                                "index": 1,
                            },
                            additional_info,
                        ]
                    },
                }
            }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info
        }
    ).data

    output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
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
    ).data

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=False, review_custom_screen_text=r"Sign\screate\stoken")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([0] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    inp = sign_tx_req_obj.encode(
        {
            "Input": 
                {
                    "addresses": [
                        {"path": [44 + h, 19788 + h, 0 + h, 0, 0], "multisig_idx": None}
                    ],
                    "inp": {
                        "Utxo": [
                            {
                                "id": {"Transaction": "0x{}".format(bytes([0] * 32).hex())},
                                "index": 1,
                            },
                            additional_info,
                        ]
                    },
                }
            }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info
        }
    ).data

    # This is the new output for issuing an NFT.
    # The structure is (TokenId, NftIssuance, Destination)
    output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
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
                }
            }
        }
    ).data

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET, inputs=[inp], input_commitments=[inp_commitment], outputs=[output]
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=False, review_custom_screen_text=r"Sign\screate\sNFT")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    utxo_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "Utxo": [
                        {
                            "id": {"Transaction": f"0x{bytes([1]*32).hex()}"},
                            "index": 0,
                        },
                        additional_info,
                    ]
                },
            }
        }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info
        }
    ).data

    # This is the AccountCommand to mint 1000 units of a new token
    account_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "AccountCommand": [
                        1,  # AccountNonce
                        {
                            "MintTokens": [
                                f"0x{bytes([0]*32).hex()}",  # TokenId
                                1000,  # Amount to mint
                            ]
                        },
                    ]
                },
            }
        }
    ).data

    acc_inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": {"None": None}
        }
    ).data

    mint_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "Transfer": [
                        {"TokenV1": [f"0x{bytes([0]*32).hex()}", 1000]},
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                            }
                        },
                    ],
                }
            }
        }
    ).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, acc_inp_commitment],
        outputs=[mint_output],
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=True, review_custom_screen_text=r"Sign\smint\stokens")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    additional_info2 = {
            "Utxo": {
                "Transfer": [
                    {"TokenV1": [f"0x{bytes([0]*32).hex()}", 1000]},
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
    utxo_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "Utxo": [
                        {
                            "id": {"Transaction": f"0x{bytes([1]*32).hex()}"},
                            "index": 0,
                        },
                        additional_info,
                    ]
                },
            }
        }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info,
        }
    ).data

    utxo_input2 = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "Utxo": [
                        {
                            "id": {"Transaction": f"0x{bytes([1]*32).hex()}"},
                            "index": 2,
                        },
                        additional_info2,
                    ]
                },
            }
        }
    ).data

    inp_commitment2 = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info2,
        }
    ).data

    account_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "AccountCommand": [
                        1,  # AccountNonce
                        {
                            "UnmintTokens": f"0x{bytes([0]*32).hex()}",  # TokenId
                        },
                    ]
                },
            }
        }
    ).data

    acc_inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": {"None": None}
        }
    ).data

    change_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "Transfer": [
                        {"Coin": 99},
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                            }
                        },
                    ],
                }
            }
        }
    ).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input, utxo_input2],
        input_commitments=[inp_commitment, acc_inp_commitment, inp_commitment2],
        outputs=[change_output],
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=True, review_custom_screen_text=r"Sign\sunmint\stokens")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }

    utxo_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "Utxo": [
                        {
                            "id": {"Transaction": f"0x{bytes([1]*32).hex()}"},
                            "index": 0,
                        },
                        additional_info,
                    ]
                },
            }
        }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info,
        }
    ).data

    # This is the AccountCommand to mint 1000 units of a new token
    account_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "AccountCommand": [
                        1,  # AccountNonce
                        {"FreezeToken": [f"0x{bytes([0]*32).hex()}", {"No": None}]},  # TokenId
                    ]
                },
            }
        }
    ).data

    acc_inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": {"None": None}
        }
    ).data

    change_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "Transfer": [
                        {"Coin": 99},
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                            }
                        },
                    ],
                }
            }
        }
    ).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, acc_inp_commitment],
        outputs=[change_output],
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=True, review_custom_screen_text=r"Sign\sfreeze\stokens")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },  
                ],
            }
        }
    utxo_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "Utxo": [
                        {
                            "id": {"Transaction": f"0x{bytes([1]*32).hex()}"},
                            "index": 0,
                        },
                        additional_info,
                    ]
                },
            }
        }).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info,
        }
    ).data

    # This is the AccountCommand to mint 1000 units of a new token
    account_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "AccountCommand": [
                        1,  # AccountNonce
                        {
                            "UnfreezeToken": f"0x{bytes([0]*32).hex()}",  # TokenId
                        },
                    ]
                },
            }
        }
    ).data

    acc_inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": {"None": None}
        }
    ).data

    change_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "Transfer": [
                        {"Coin": 99},
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                            }
                        },
                    ],
                }
            }
        }
    ).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, acc_inp_commitment],
        outputs=[change_output],
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=True, review_custom_screen_text=r"Sign\sunfreeze")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    utxo_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "Utxo": [
                        {
                            "id": {"Transaction": f"0x{bytes([1]*32).hex()}"},
                            "index": 0,
                        },
                        additional_info,
                    ]
                },
            }
        }).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info,
        }
    ).data

    # This is the AccountCommand to mint 1000 units of a new token
    account_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "AccountCommand": [
                        1,  # AccountNonce
                        {
                            "ChangeTokenAuthority": [
                                f"0x{bytes([0]*32).hex()}",  # TokenId
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
        }
    ).data

    acc_inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": {"None": None}
        }
    ).data

    change_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "Transfer": [
                        {"Coin": 99},
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                            }
                        },
                    ],
                }
            }
        }
    ).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, acc_inp_commitment],
        outputs=[change_output],
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=True, review_custom_screen_text=r"Sign\schange\stoken")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    utxo_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "Utxo": [
                        {
                            "id": {"Transaction": f"0x{bytes([1]*32).hex()}"},
                            "index": 0,
                        },
                        additional_info,
                    ]
                },
            }
        }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info,
        }
    ).data

    # This is the AccountCommand to mint 1000 units of a new token
    account_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "AccountCommand": [
                        1,  # AccountNonce
                        {
                            "ChangeTokenMetadataUri": [
                                f"0x{bytes([0]*32).hex()}",  # TokenId
                                "uri.com".encode(),
                            ]
                        },
                    ]
                },
            }
        }
    ).data

    acc_inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": {"None": None}
        }
    ).data

    change_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "Transfer": [
                        {"Coin": 99},
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                            }
                        },
                    ],
                }
            }
        }
    ).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, acc_inp_commitment],
        outputs=[change_output],
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=True, review_custom_screen_text=r"Sign\schange\stoken")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    utxo_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "Utxo": [
                        {
                            "id": {"Transaction": f"0x{bytes([1]*32).hex()}"},
                            "index": 0,
                        },
                        additional_info,
                    ]
                },
            }
        }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info,
        }
    ).data

    fill_amount = 10
    fill_ask = 100
    fill_give = 1000

    additional_order_info = {
           "initially_asked": {"Coin": fill_ask},
           "initially_given": {"TokenV1": [f"0x{bytes([0]*32).hex()}", fill_give]},
           "ask_balance": 0,
           "give_balance": 0,
    }
    # This is the OrderAccountCommand to fill 10 units
    account_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "OrderAccountCommand": [
                        {
                            "FillOrder": [
                                f"0x{bytes([0]*32).hex()}",  # OrderId
                                fill_amount,
                            ]
                        },
                        additional_order_info,
                    ]
                },
            }
        }
    ).data

    fill_order_inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": {
                "FillOrderAccountCommand": [
                    additional_order_info["initially_asked"],
                    additional_order_info["initially_given"],
                ]
            },
        }
    ).data

    change_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "Transfer": [
                        {"Coin": 100 - 1 - fill_amount},
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                            }
                        },
                    ],
                }
            }
        }
    ).data

    fill_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "Transfer": [
                        {
                            "TokenV1": [
                                f"0x{bytes([0]*32).hex()}",
                                fill_amount * fill_give // fill_ask,
                            ]
                        },
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                            }
                        },
                    ],
                }
            }
        }
    ).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, fill_order_inp_commitment],
        outputs=[change_output, fill_output],
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=True, review_custom_screen_text=r"Sign\sfill\sorder")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    utxo_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "Utxo": [
                        {
                            "id": {"Transaction": f"0x{bytes([1]*32).hex()}"},
                            "index": 0,
                        },
                        additional_info,
                    ]
                },
            }
        }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info,
        }
    ).data

    initial_ask = 100
    initial_give = 1000
    ask_balance = 10
    give_balance = 900

    additional_order_info = {
           "initially_asked": {"Coin": initial_ask},
           "initially_given": {"TokenV1": [f"0x{bytes([0]*32).hex()}", initial_give]},
           "ask_balance": ask_balance,
           "give_balance": give_balance,
    }

    # This is the OrderAccountCommand to fill 10 units
    account_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "OrderAccountCommand": [
                        {
                            "ConcludeOrder": f"0x{bytes([0]*32).hex()}",  # OrderId
                        },
                        additional_order_info,
                    ]
                },
            }
        }
    ).data

    conclude_order_inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": {
                "ConcludeOrderAccountCommand": [
                    additional_order_info["initially_asked"],
                    additional_order_info["initially_given"],
                    additional_order_info["ask_balance"],
                    additional_order_info["give_balance"]
                ]
            },
        }
    ).data

    change_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "Transfer": [
                        {"Coin": 100 - 1 + ask_balance},
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                            }
                        },
                    ],
                }
            }
        }
    ).data

    conclude_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "Transfer": [
                        {"TokenV1": [f"0x{bytes([0]*32).hex()}", give_balance]},
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                            }
                        },
                    ],
                }
            }
        }
    ).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitments=[inp_commitment, conclude_order_inp_commitment],
        outputs=[change_output, conclude_output],
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=True, review_custom_screen_text=r"Sign\sconclude\sorder")


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
                            "key": {
                                "Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}
                            }
                        }
                    },
                ],
            }
        }
    utxo_input = sign_tx_req_obj.encode(
        {
            "Input": {
                "addresses": [{"path": bip44_path, "multisig_idx": None}],
                "inp": {
                    "Utxo": [
                        {
                            "id": {"Transaction": f"0x{bytes([1]*32).hex()}"},
                            "index": 0,
                        },
                        additional_info,
                    ]
                },
            }
        }
    ).data

    inp_commitment = sign_tx_req_obj.encode(
        {
            "InputCommitment": additional_info,
        }
    ).data

    change_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
                    "Transfer": [
                        {"Coin": 89},
                        {
                            "PublicKey": {
                                "key": {"Secp256k1Schnorr": {"pubkey_data": bytes([2] * 33)}}
                            }
                        },
                    ],
                }
            }
        }
    ).data

    htlc_output = sign_tx_req_obj.encode(
        {
            "Output": {
                "out": {
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
    ).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input],
        input_commitments=[inp_commitment],
        outputs=[htlc_output, change_output],
    )

    sign_tx_review(client, device, navigator, scenario_navigator, transaction, has_command_input=False, review_custom_screen_text=r"Sign\screate\sHTLC")
