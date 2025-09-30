import pytest
import scalecodec

from application_client.boilerplate_transaction import Transaction
from application_client.boilerplate_command_sender import BoilerplateCommandSender, Errors
from application_client.boilerplate_response_unpacker import unpack_get_public_key_response, unpack_sign_tx_response
from application_client import MAINNET, TESTNET
from ragger.error import ExceptionRAPDU
from ragger.navigator import NavIns, NavInsID
from utils import ROOT_SCREENSHOT_PATH, check_signature_validity

input_meta_obj = scalecodec.base.RuntimeConfiguration().create_scale_object('InputMeta')
tx_input_obj = scalecodec.base.RuntimeConfiguration().create_scale_object('TxInput')
commitement_obj = scalecodec.base.RuntimeConfiguration().create_scale_object('SighashInputCommitment')
output_obj = scalecodec.base.RuntimeConfiguration().create_scale_object('TxOutput')

TX_RESPONSE_SIZE = 67

def test_sign_tx_transfer(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    # The path used for this entire test
    path: str = "m/44'/19788'/0'/0/0"

    # First we need to get the public key of the device in order to build the transaction
    rapdu = client.get_public_key(coin=MAINNET, path=path)
    _, public_key, _, _ = unpack_get_public_key_response(rapdu.data)

    print("pk", len(public_key))

    h = 1<<31
    inp = (input_meta_obj.encode({
        "addresses": [
            {
                "path": [44+h, 19788+h, 0+h, 0, 0],
                "multisig_idx": None
            }
        ]
    }).data, tx_input_obj.encode({ 'Utxo' : {
           'id': { 'Transaction': '0x{}'.format(bytes([0]*32).hex()) },
           'index': 1,
        }
    }).data)

    commitement = commitement_obj.encode({ 'Utxo': {
        'Transfer': [ { 'Coin': 10 }, { 'PublicKey': {'key': {'Secp256k1Schnorr' : {'pubkey_data': bytes([0]*33)}}} } ],
    }}).data

    output = output_obj.encode({
        'Transfer': [ { 'Coin': 10 }, { 'PublicKey': {'key': {'Secp256k1Schnorr' : {'pubkey_data': bytes([0]*33)}}} } ],
    }).data
    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET,
        inputs=[inp],
        input_commitements=[commitement],
        outputs=[output]
    )
    
    # Enable display of transaction memo (NBGL devices only)
    if not device.is_nano:
        navigator.navigate([NavInsID.USE_CASE_HOME_SETTINGS,
                            NavIns(NavInsID.TOUCH, (200, 113)),
                            NavInsID.USE_CASE_SUB_SETTINGS_EXIT],
                            screen_change_before_first_instruction=False,
                            screen_change_after_last_instruction=False)

    # Send the sign device instruction.
    # As it requires on-screen validation, the function is asynchronous.
    # It will yield the result when the navigation is done
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation appropriate for this device
        scenario_navigator.review_approve(custom_screen_text=r"Sign\stransfer")

    # The device as yielded the result, parse it and ensure that the signature is correct
    response = client.get_async_response().data

    assert len(response) == TX_RESPONSE_SIZE
    #_, der_sig, _ = unpack_sign_tx_response(response)
    
    #assert check_signature_validity(public_key, der_sig, transaction.to_hash())
    
def test_sign_tx_lock_then_transfer(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    # The path used for this entire test
    path: str = "m/44'/19788'/0'/0/0"

    h = 1<<31
    inp = (input_meta_obj.encode({
        "addresses": [
            {
                "path": [44+h, 19788+h, 0+h, 0, 0],
                "multisig_idx": None
            }
        ]
    }).data, tx_input_obj.encode({ 'Account' : {
            'nonce': 1,
            'account': {
                'Delegation': [[0]*32, 11]
            },
        }
    }).data)

    commitement = commitement_obj.encode({'None': None}).data

    output = output_obj.encode({
        'LockThenTransfer': [ { 'Coin': 10 }, { 'PublicKey': {'key': {'Secp256k1Schnorr' : {'pubkey_data': bytes([0]*33)}}} }, {'UntilHeight': 10} ],
    }).data
    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET,
        inputs=[inp],
        input_commitements=[commitement],
        outputs=[output]
    )
    
    # Enable display of transaction memo (NBGL devices only)
    if not device.is_nano:
        navigator.navigate([NavInsID.USE_CASE_HOME_SETTINGS,
                            NavIns(NavInsID.TOUCH, (200, 113)),
                            NavInsID.USE_CASE_SUB_SETTINGS_EXIT],
                            screen_change_before_first_instruction=False,
                            screen_change_after_last_instruction=False)

    # Send the sign device instruction.
    # As it requires on-screen validation, the function is asynchronous.
    # It will yield the result when the navigation is done
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation appropriate for this device
        scenario_navigator.review_approve(custom_screen_text=r"Sign\swithdrawal")

    # The device as yielded the result, parse it and ensure that the signature is correct
    response = client.get_async_response().data

    assert len(response) == TX_RESPONSE_SIZE

def test_sign_tx_create_delegation(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    # The path used for this entire test
    path: str = "m/44'/19788'/0'/0/0"

    h = 1<<31
    inp = (input_meta_obj.encode({
        "addresses": [
            {
                "path": [44+h, 19788+h, 0+h, 0, 0],
                "multisig_idx": None
            }
        ]
    }).data, tx_input_obj.encode({ 'Utxo' : {
           'id': { 'Transaction': '0x{}'.format(bytes([0]*32).hex()) },
           'index': 1,
        }
    }).data)

    commitement = commitement_obj.encode({ 'Utxo': {
        'Transfer': [ { 'Coin': 10 }, { 'PublicKey': {'key': {'Secp256k1Schnorr' : {'pubkey_data': bytes([0]*33)}}} } ],
    }}).data

    output = output_obj.encode({
        'CreateDelegationId': [ { 'PublicKey': {'key': {'Secp256k1Schnorr' : {'pubkey_data': bytes([0]*33)}}} }, [0]*32 ],
    }).data
    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET,
        inputs=[inp],
        input_commitements=[commitement],
        outputs=[output]
    )
    
    # Enable display of transaction memo (NBGL devices only)
    if not device.is_nano:
        navigator.navigate([NavInsID.USE_CASE_HOME_SETTINGS,
                            NavIns(NavInsID.TOUCH, (200, 113)),
                            NavInsID.USE_CASE_SUB_SETTINGS_EXIT],
                            screen_change_before_first_instruction=False,
                            screen_change_after_last_instruction=False)

    print("create delegation test")
    # Send the sign device instruction.
    # As it requires on-screen validation, the function is asynchronous.
    # It will yield the result when the navigation is done
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation appropriate for this device
        scenario_navigator.review_approve(custom_screen_text=r"Sign\screate")

    # The device as yielded the result, parse it and ensure that the signature is correct
    response = client.get_async_response().data

    assert len(response) == TX_RESPONSE_SIZE

def test_sign_tx_delegation_staking(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    # The path used for this entire test
    path: str = "m/44'/19788'/0'/0/0"

    h = 1<<31
    inp = (input_meta_obj.encode({
        "addresses": [
            {
                "path": [44+h, 19788+h, 0+h, 0, 0],
                "multisig_idx": None
            }
        ]
    }).data, tx_input_obj.encode({ 'Utxo' : {
           'id': { 'Transaction': '0x{}'.format(bytes([0]*32).hex()) },
           'index': 1,
        }
    }).data)

    commitement = commitement_obj.encode({ 'Utxo': {
        'Transfer': [ { 'Coin': 10 }, { 'PublicKey': {'key': {'Secp256k1Schnorr' : {'pubkey_data': bytes([0]*33)}}} } ],
    }}).data

    output = output_obj.encode({
        'DelegateStaking': [ 5, [0]*32 ],
    }).data
    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET,
        inputs=[inp],
        input_commitements=[commitement],
        outputs=[output]
    )
    
    # Enable display of transaction memo (NBGL devices only)
    if not device.is_nano:
        navigator.navigate([NavInsID.USE_CASE_HOME_SETTINGS,
                            NavIns(NavInsID.TOUCH, (200, 113)),
                            NavInsID.USE_CASE_SUB_SETTINGS_EXIT],
                            screen_change_before_first_instruction=False,
                            screen_change_after_last_instruction=False)

    # Send the sign device instruction.
    # As it requires on-screen validation, the function is asynchronous.
    # It will yield the result when the navigation is done
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation appropriate for this device
        scenario_navigator.review_approve(custom_screen_text=r"Sign\sstake")

    # The device as yielded the result, parse it and ensure that the signature is correct
    response = client.get_async_response().data

    assert len(response) == TX_RESPONSE_SIZE

def test_sign_tx_create_stake_pool(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    # The path used for this entire test
    path: str = "m/44'/19788'/0'/0/0"

    h = 1<<31
    inp = (input_meta_obj.encode({
        "addresses": [
            {
                "path": [44+h, 19788+h, 0+h, 0, 0],
                "multisig_idx": None
            }
        ]
    }).data, tx_input_obj.encode({ 'Utxo' : {
           'id': { 'Transaction': '0x{}'.format(bytes([0]*32).hex()) },
           'index': 1,
        }
    }).data)

    commitement = commitement_obj.encode({ 'Utxo': {
        'Transfer': [ { 'Coin': 40001 }, { 'PublicKey': {'key': {'Secp256k1Schnorr' : {'pubkey_data': bytes([0]*33)}}} } ],
    }}).data

    output = output_obj.encode({
        'CreateStakePool': [ [0]*32, {
            'value': 40000,
            'staker': { 'PublicKey': {'key': {'Secp256k1Schnorr' : {'pubkey_data': bytes([0]*33)}}} },
            'vrf_public_key': {'key': {'Schnorrkel': {'key': bytes([0]*32)}}},
            'decommission_key': { 'PublicKey': {'key': {'Secp256k1Schnorr' : {'pubkey_data': bytes([0]*33)}}} },
            'margin_ratio_per_thousand': 10,
            'cost_per_block': 5
        } ],
    }).data
    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET,
        inputs=[inp],
        input_commitements=[commitement],
        outputs=[output]
    )
    
    # Enable display of transaction memo (NBGL devices only)
    if not device.is_nano:
        navigator.navigate([NavInsID.USE_CASE_HOME_SETTINGS,
                            NavIns(NavInsID.TOUCH, (200, 113)),
                            NavInsID.USE_CASE_SUB_SETTINGS_EXIT],
                            screen_change_before_first_instruction=False,
                            screen_change_after_last_instruction=False)

    # Send the sign device instruction.
    # As it requires on-screen validation, the function is asynchronous.
    # It will yield the result when the navigation is done
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation appropriate for this device
        scenario_navigator.review_approve(custom_screen_text=r"Sign\screate\sstake")

    # The device as yielded the result, parse it and ensure that the signature is correct
    response = client.get_async_response().data

    assert len(response) == TX_RESPONSE_SIZE

def test_sign_tx_issue_fungible_token(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    # The path used for this entire test
    path: str = "m/44'/19788'/0'/0/0"

    
    h = 1 << 31
    inp = (input_meta_obj.encode({
        "addresses": [
            {
                "path": [44+h, 19788+h, 0+h, 0, 0],
                "multisig_idx": None
            }
        ]
    }).data, tx_input_obj.encode({'Utxo': {
        'id': {'Transaction': '0x{}'.format(bytes([0] * 32).hex())},
        'index': 1,
    }
    }).data)

    commitement = commitement_obj.encode({ 'Utxo': {
        'Transfer': [{'Coin': 1000}, {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([0] * 33)}}}}],
    }}).data

    output = output_obj.encode({
        'IssueFungibleToken': {
            'V1': {
                'token_ticker': b'MYTKN',
                'number_of_decimals': 8,
                'metadata_uri': b'https://my.token.uri',
                'total_supply': {'Fixed': 1000000000},
                'authority': {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([0] * 33)}}}},
                'is_freezable': {'Yes': None},
            }
        },
    }).data

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET,
        inputs=[inp],
        input_commitements=[commitement],
        outputs=[output]
    )

    # Send the sign device instruction
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request
        scenario_navigator.review_approve(custom_screen_text=r"Sign\screate\sToken")

    # The device has yielded the result, parse it and ensure that the signature is correct
    response = client.get_async_response().data
    assert len(response) == TX_RESPONSE_SIZE

def test_sign_tx_issue_nft(backend, scenario_navigator, device, navigator):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    # The path used for this entire test
    path: str = "m/44'/19788'/0'/0/0"

    h = 1 << 31
    inp = (input_meta_obj.encode({
        "addresses": [
            {
                "path": [44+h, 19788+h, 0+h, 0, 0],
                "multisig_idx": None
            }
        ]
    }).data, tx_input_obj.encode({'Utxo': {
        'id': {'Transaction': '0x{}'.format(bytes([0] * 32).hex())},
        'index': 1,
    }
    }).data)

    commitement = commitement_obj.encode({ 'Utxo': {
        'Transfer': [{'Coin': 2000}, {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([0] * 33)}}}}],
    }}).data

    # This is the new output for issuing an NFT.
    # The structure is (TokenId, NftIssuance, Destination)
    output = output_obj.encode({
        'IssueNft': [
            bytes([0] * 32),
            {
                'V0': {
                    'metadata': {
                        'creator': {'public_key': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([0] * 33)}}}},
                        'name': b'MyAwesomeNFT',
                        'description': b'FirstNFT',
                        'ticker': b'MNFT1',
                        'icon_uri': b'https://my.nft/icon.png',
                        'additional_metadata_uri': b'https://my.nft/meta.json',
                        'media_uri': b'https://my.nft/media.jpg',
                        'media_hash': bytes([0] * 32),
                    }
                }
            },
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([0] * 33)}}}},
        ],
    }).data

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        coin=MAINNET,
        inputs=[inp],
        input_commitements=[commitement],
        outputs=[output]
    )

    # Send the sign device instruction
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request
        scenario_navigator.review_approve(custom_screen_text=r"Sign\screate\sNFT")

    # The device has yielded the result, parse it and ensure that the signature is correct
    response = client.get_async_response().data
    assert len(response) == TX_RESPONSE_SIZE

def test_sign_tx_mint_tokens(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input.
    2. An AccountCommand input to mint new tokens.
    And one output to transfer the newly minted tokens.
    """
    client = BoilerplateCommandSender(backend)
    # The path for the key that will sign the inputs
    path: str = "m/44'/19788'/0'/0/0"
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    utxo_input_data = tx_input_obj.encode({
        'Utxo': {
            'id': {'Transaction': f'0x{bytes([1]*32).hex()}'},
            'index': 0,
        }
    }).data

    # The commitment (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    utxo_commitment = commitement_obj.encode({ 'Utxo': {
        'Transfer': [
            {'Coin': 100},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }}).data

    # The complete UTXO input tuple (meta, data)
    utxo_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        utxo_input_data
    )

    # This is the AccountCommand to mint 1000 units of a new token
    account_input_data = tx_input_obj.encode({
        'AccountCommand': [
            1,  # AccountNonce
            {
                'MintTokens': [
                    f'0x{bytes([0]*32).hex()}', # TokenId
                    1000                       # Amount to mint
                ]
            }
        ]
    }).data

    account_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        account_input_data
    )

    mint_output = output_obj.encode({
        'Transfer': [
            {'TokenV1': [f'0x{bytes([0]*32).hex()}', 1000]},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitements=[utxo_commitment, commitement_obj.encode({'None': None}).data],
        outputs=[mint_output]
    )

    # Send the sign transaction instruction.
    # It will yield the result when the user validates on-screen.
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation
        scenario_navigator.review_approve(custom_screen_text=r"Sign\smint\sTokens")
    # The device has yielded the result, parse it and ensure the signatures are correct
    responses = client.get_all_signatures(transaction)

    # The device should have returned two signatures, one for each input that
    # required signing (the Utxo and the AccountCommand).
    # Each signature is 64 bytes + 3 sighash byte = 67 bytes.
    assert len(responses) == 2
    for resp in responses:
        assert len(resp) == TX_RESPONSE_SIZE

def test_sign_tx_unmint_tokens(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay tx fees.
    2. An AccountCommand input to unmint new tokens.
    3. A standard UTXO input with tokens to unmint
    And one output to transfer the newly minted tokens.
    """
    client = BoilerplateCommandSender(backend)
    # The path for the key that will sign the inputs
    path: str = "m/44'/19788'/0'/0/0"
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    utxo_input_data = tx_input_obj.encode({
        'Utxo': {
            'id': {'Transaction': f'0x{bytes([1]*32).hex()}'},
            'index': 0,
        }
    }).data

    utxo_input_data2 = tx_input_obj.encode({
        'Utxo': {
            'id': {'Transaction': f'0x{bytes([1]*32).hex()}'},
            'index': 2,
        }
    }).data

    # The commitment (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    utxo_commitment = commitement_obj.encode({ 'Utxo': {
        'Transfer': [
            {'Coin': 100},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }}).data

    utxo_commitment2 = commitement_obj.encode({ 'Utxo': {
        'Transfer': [
             {'TokenV1': [f'0x{bytes([0]*32).hex()}', 1000]},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }}).data

    # The complete UTXO input tuple (meta, data)
    utxo_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        utxo_input_data
    )
    utxo_input2 = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        utxo_input_data2
    )
    
    account_input_data = tx_input_obj.encode({
        'AccountCommand': [
            1,  # AccountNonce
            {
                'UnmintTokens': f'0x{bytes([0]*32).hex()}', # TokenId
            }
        ]
    }).data

    account_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        account_input_data
    )

    change_output = output_obj.encode({
        'Transfer': [
            {'Coin': 99},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input, utxo_input2],
        input_commitements=[utxo_commitment, commitement_obj.encode({'None': None}).data, utxo_commitment2],
        outputs=[change_output]
    )

    # Send the sign transaction instruction.
    # It will yield the result when the user validates on-screen.
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation
        scenario_navigator.review_approve(custom_screen_text=r"Sign\sunmint\sTokens")
    # The device has yielded the result, parse it and ensure the signatures are correct
    responses = client.get_all_signatures(transaction)

    # The device should have returned two signatures, one for each input that
    # required signing (the Utxo and the AccountCommand).
    # Each signature is 64 bytes + 3 sighash byte = 67 bytes.
    assert len(responses) == 3
    for resp in responses:
        assert len(resp) == TX_RESPONSE_SIZE

def test_sign_tx_freeze_tokens(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An AccountCommand input to freeze the tokens.
    And one output to transfer the change coins.
    """
    client = BoilerplateCommandSender(backend)
    # The path for the key that will sign the inputs
    path: str = "m/44'/19788'/0'/0/0"
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    utxo_input_data = tx_input_obj.encode({
        'Utxo': {
            'id': {'Transaction': f'0x{bytes([1]*32).hex()}'},
            'index': 0,
        }
    }).data

    # The commitment (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    utxo_commitment = commitement_obj.encode({ 'Utxo': {
        'Transfer': [
            {'Coin': 100},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }}).data

    # The complete UTXO input tuple (meta, data)
    utxo_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        utxo_input_data
    )

    # This is the AccountCommand to mint 1000 units of a new token
    account_input_data = tx_input_obj.encode({
        'AccountCommand': [
            1,  # AccountNonce
            {
                'FreezeToken': [
                    f'0x{bytes([0]*32).hex()}', # TokenId
                    {'No': None}
                ]
            }
        ]
    }).data

    account_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        account_input_data
    )

    change_output = output_obj.encode({
        'Transfer': [
            {'Coin': 99},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitements=[utxo_commitment, commitement_obj.encode({'None': None}).data],
        outputs=[change_output]
    )

    # Send the sign transaction instruction.
    # It will yield the result when the user validates on-screen.
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation
        scenario_navigator.review_approve(custom_screen_text=r"Sign\sfreeze\sTokens")
    # The device has yielded the result, parse it and ensure the signatures are correct
    responses = client.get_all_signatures(transaction)

    # The device should have returned two signatures, one for each input that
    # required signing (the Utxo and the AccountCommand).
    # Each signature is 64 bytes + 3 sighash byte = 67 bytes.
    assert len(responses) == 2
    for resp in responses:
        assert len(resp) == TX_RESPONSE_SIZE

def test_sign_tx_unfreeze_tokens(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An AccountCommand input to unfreeze the tokens.
    And one output to transfer the change coins.
    """
    client = BoilerplateCommandSender(backend)
    # The path for the key that will sign the inputs
    path: str = "m/44'/19788'/0'/0/0"
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    utxo_input_data = tx_input_obj.encode({
        'Utxo': {
            'id': {'Transaction': f'0x{bytes([1]*32).hex()}'},
            'index': 0,
        }
    }).data

    # The commitment (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    utxo_commitment = commitement_obj.encode({ 'Utxo': {
        'Transfer': [
            {'Coin': 100},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }}).data

    # The complete UTXO input tuple (meta, data)
    utxo_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        utxo_input_data
    )

    # This is the AccountCommand to mint 1000 units of a new token
    account_input_data = tx_input_obj.encode({
        'AccountCommand': [
            1,  # AccountNonce
            {
                'UnfreezeToken': f'0x{bytes([0]*32).hex()}', # TokenId
            }
        ]
    }).data

    account_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        account_input_data
    )

    change_output = output_obj.encode({
        'Transfer': [
            {'Coin': 99},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitements=[utxo_commitment, commitement_obj.encode({'None': None}).data],
        outputs=[change_output]
    )

    # Send the sign transaction instruction.
    # It will yield the result when the user validates on-screen.
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation
        scenario_navigator.review_approve(custom_screen_text=r"Sign\sunfreeze")
    # The device has yielded the result, parse it and ensure the signatures are correct
    responses = client.get_all_signatures(transaction)

    # The device should have returned two signatures, one for each input that
    # required signing (the Utxo and the AccountCommand).
    # Each signature is 64 bytes + 3 sighash byte = 67 bytes.
    assert len(responses) == 2
    for resp in responses:
        assert len(resp) == TX_RESPONSE_SIZE

def test_sign_tx_change_token_authority(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An AccountCommand input to change the token's authority.
    And one output to transfer the change coins.
    """
    client = BoilerplateCommandSender(backend)
    # The path for the key that will sign the inputs
    path: str = "m/44'/19788'/0'/0/0"
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    utxo_input_data = tx_input_obj.encode({
        'Utxo': {
            'id': {'Transaction': f'0x{bytes([1]*32).hex()}'},
            'index': 0,
        }
    }).data

    # The commitment (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    utxo_commitment = commitement_obj.encode({ 'Utxo': {
        'Transfer': [
            {'Coin': 100},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }}).data

    # The complete UTXO input tuple (meta, data)
    utxo_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        utxo_input_data
    )

    # This is the AccountCommand to mint 1000 units of a new token
    account_input_data = tx_input_obj.encode({
        'AccountCommand': [
            1,  # AccountNonce
            {
                'ChangeTokenAuthority': [
                    f'0x{bytes([0]*32).hex()}', # TokenId
                    {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
                ]
            }
        ]
    }).data

    account_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        account_input_data
    )

    change_output = output_obj.encode({
        'Transfer': [
            {'Coin': 99},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitements=[utxo_commitment, commitement_obj.encode({'None': None}).data],
        outputs=[change_output]
    )

    # Send the sign transaction instruction.
    # It will yield the result when the user validates on-screen.
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation
        scenario_navigator.review_approve(custom_screen_text=r"Sign\schange\sToken")
    # The device has yielded the result, parse it and ensure the signatures are correct
    responses = client.get_all_signatures(transaction)

    # The device should have returned two signatures, one for each input that
    # required signing (the Utxo and the AccountCommand).
    # Each signature is 64 bytes + 3 sighash byte = 67 bytes.
    assert len(responses) == 2
    for resp in responses:
        assert len(resp) == TX_RESPONSE_SIZE

def test_sign_tx_change_token_metadata_uri(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An AccountCommand input to change the token's metadata uri.
    And one output to transfer the change coins.
    """
    client = BoilerplateCommandSender(backend)
    # The path for the key that will sign the inputs
    path: str = "m/44'/19788'/0'/0/0"
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    utxo_input_data = tx_input_obj.encode({
        'Utxo': {
            'id': {'Transaction': f'0x{bytes([1]*32).hex()}'},
            'index': 0,
        }
    }).data

    # The commitment (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    utxo_commitment = commitement_obj.encode({ 'Utxo': {
        'Transfer': [
            {'Coin': 100},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }}).data

    # The complete UTXO input tuple (meta, data)
    utxo_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        utxo_input_data
    )

    # This is the AccountCommand to mint 1000 units of a new token
    account_input_data = tx_input_obj.encode({
        'AccountCommand': [
            1,  # AccountNonce
            {
                'ChangeTokenMetadataUri': [
                    f'0x{bytes([0]*32).hex()}', # TokenId
                    "uri.com".encode()
                ]
            }
        ]
    }).data

    account_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        account_input_data
    )

    change_output = output_obj.encode({
        'Transfer': [
            {'Coin': 99},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitements=[utxo_commitment, commitement_obj.encode({'None': None}).data],
        outputs=[change_output]
    )

    # Send the sign transaction instruction.
    # It will yield the result when the user validates on-screen.
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation
        scenario_navigator.review_approve(custom_screen_text=r"Sign\schange\sToken")
    # The device has yielded the result, parse it and ensure the signatures are correct
    responses = client.get_all_signatures(transaction)

    # The device should have returned two signatures, one for each input that
    # required signing (the Utxo and the AccountCommand).
    # Each signature is 64 bytes + 3 sighash byte = 67 bytes.
    assert len(responses) == 2
    for resp in responses:
        assert len(resp) == TX_RESPONSE_SIZE

def test_sign_tx_order_fill(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An AccountCommand input to fill an order.
    3. A standard UTXO input to be used for the fill.
    And one output to transfer the change coins.
    """
    client = BoilerplateCommandSender(backend)
    # The path for the key that will sign the inputs
    path: str = "m/44'/19788'/0'/0/0"
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    utxo_input_data = tx_input_obj.encode({
        'Utxo': {
            'id': {'Transaction': f'0x{bytes([1]*32).hex()}'},
            'index': 0,
        }
    }).data

    # The commitment (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    utxo_commitment = commitement_obj.encode({ 'Utxo': {
        'Transfer': [
            {'Coin': 100},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }}).data

    # The complete UTXO input tuple (meta, data)
    utxo_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        utxo_input_data
    )

    fill_amount = 10
    # This is the OrderAccountCommand to fill 10 units
    account_input_data = tx_input_obj.encode({
        'OrderAccountCommand':
            {
                'FillOrder': [
                    f'0x{bytes([0]*32).hex()}', # OrderId
                    fill_amount,
                ]
            }
    }).data

    fill_ask = 100
    fill_give = 1000
    fill_comitment = commitement_obj.encode({'FillOrderAccountCommand': [
        {'Coin': fill_ask},
        {'TokenV1': [f'0x{bytes([0]*32).hex()}', fill_give]}
    ]}).data


    account_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        account_input_data
    )

    change_output = output_obj.encode({
        'Transfer': [
            {'Coin': 100 - 1 - fill_amount},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }).data

    fill_output = output_obj.encode({
        'Transfer': [
            {'TokenV1': [f'0x{bytes([0]*32).hex()}', fill_amount * fill_give // fill_ask ]},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }).data


    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitements=[utxo_commitment, fill_comitment],
        outputs=[change_output, fill_output]
    )

    # Send the sign transaction instruction.
    # It will yield the result when the user validates on-screen.
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation
        scenario_navigator.review_approve(custom_screen_text=r"Sign\sfill\sOrder")
    # The device has yielded the result, parse it and ensure the signatures are correct
    responses = client.get_all_signatures(transaction)

    # The device should have returned two signatures, one for each input that
    # required signing (the Utxo and the AccountCommand).
    # Each signature is 64 bytes + 3 sighash byte = 67 bytes.
    assert len(responses) == 2
    for resp in responses:
        assert len(resp) == TX_RESPONSE_SIZE

def test_sign_tx_order_conclude(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    2. An OrderAccountCommand input to conclude an order.
    And one output to transfer the change coins + ask balance and another output for the give balance.
    """
    client = BoilerplateCommandSender(backend)
    # The path for the key that will sign the inputs
    path: str = "m/44'/19788'/0'/0/0"
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    utxo_input_data = tx_input_obj.encode({
        'Utxo': {
            'id': {'Transaction': f'0x{bytes([1]*32).hex()}'},
            'index': 0,
        }
    }).data

    # The commitment (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    utxo_commitment = commitement_obj.encode({ 'Utxo': {
        'Transfer': [
            {'Coin': 100},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }}).data

    # The complete UTXO input tuple (meta, data)
    utxo_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        utxo_input_data
    )

    # This is the OrderAccountCommand to fill 10 units
    account_input_data = tx_input_obj.encode({
        'OrderAccountCommand':
            {
                'ConcludeOrder': f'0x{bytes([0]*32).hex()}', # OrderId
            }
    }).data

    initial_ask = 100
    initial_give = 1000
    ask_balance = 10
    give_balance = 900
    conclude_comitment = commitement_obj.encode({'ConcludeOrderAccountCommand': [
        {'Coin': initial_ask},
        {'TokenV1': [f'0x{bytes([0]*32).hex()}', initial_give]},
        ask_balance,
        give_balance,
    ]}).data


    account_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        account_input_data
    )

    change_output = output_obj.encode({
        'Transfer': [
            {'Coin': 100 - 1 + ask_balance},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }).data

    conclude_output = output_obj.encode({
        'Transfer': [
            {'TokenV1': [f'0x{bytes([0]*32).hex()}', give_balance]},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }).data


    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitements=[utxo_commitment, conclude_comitment],
        outputs=[change_output, conclude_output]
    )

    # Send the sign transaction instruction.
    # It will yield the result when the user validates on-screen.
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation
        scenario_navigator.review_approve(custom_screen_text=r"Sign\sconclude\sOrder")
    # The device has yielded the result, parse it and ensure the signatures are correct
    responses = client.get_all_signatures(transaction)

    # The device should have returned two signatures, one for each input that
    # required signing (the Utxo and the AccountCommand).
    # Each signature is 64 bytes + 3 sighash byte = 67 bytes.
    assert len(responses) == 2
    for resp in responses:
        assert len(resp) == TX_RESPONSE_SIZE

def test_sign_tx_htlc(backend, scenario_navigator, device, navigator):
    """
    Test signing a transaction with two inputs:
    1. A standard UTXO input to pay for tx fees.
    And one output to transfer the change coins and the HTLC output.
    """
    client = BoilerplateCommandSender(backend)
    # The path for the key that will sign the inputs
    path: str = "m/44'/19788'/0'/0/0"
    h = 1 << 31
    bip44_path = [44 + h, 19788 + h, 0 + h, 0, 0]

    utxo_input_data = tx_input_obj.encode({
        'Utxo': {
            'id': {'Transaction': f'0x{bytes([1]*32).hex()}'},
            'index': 0,
        }
    }).data

    # The commitment (the previous TxOutput that this UTXO input spends)
    # This represents an output of 100 coins owned by our key
    utxo_commitment = commitement_obj.encode({ 'Utxo': {
        'Transfer': [
            {'Coin': 100},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }}).data

    # The complete UTXO input tuple (meta, data)
    utxo_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        utxo_input_data
    )

    # This is the AccountCommand to mint 1000 units of a new token
    account_input_data = tx_input_obj.encode({
        'AccountCommand': [
            1,  # AccountNonce
            {
                'ChangeTokenMetadataUri': [
                    f'0x{bytes([0]*32).hex()}', # TokenId
                    "uri.com".encode()
                ]
            }
        ]
    }).data

    account_input = (
        input_meta_obj.encode({
            "addresses": [
                {
                    "path": bip44_path,
                    "multisig_idx": None
                }
            ]
        }).data,
        account_input_data
    )

    change_output = output_obj.encode({
        'Transfer': [
            {'Coin': 89},
            {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}}
        ],
    }).data

    htlc_output = output_obj.encode({
        'Htlc': [
            {'Coin': 10},
            {
                'secret_hash': [0]*20,
                'spend_key': {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([2]*33)}}}},
                'refund_timelock': { 'UntilHeight': 100 },
                'refund_key': {'PublicKey': {'key': {'Secp256k1Schnorr': {'pubkey_data': bytes([3]*33)}}}},
            },
            
        ],
    }).data

    transaction = Transaction(
        coin=MAINNET,
        inputs=[utxo_input, account_input],
        input_commitements=[utxo_commitment, commitement_obj.encode({'None': None}).data],
        outputs=[htlc_output, change_output]
    )

    # Send the sign transaction instruction.
    # It will yield the result when the user validates on-screen.
    with client.sign_tx(transaction=transaction):
        # Validate the on-screen request by performing the navigation
        scenario_navigator.review_approve()
    # The device has yielded the result, parse it and ensure the signatures are correct
    responses = client.get_all_signatures(transaction)

    # The device should have returned two signatures, one for each input that
    # required signing (the Utxo and the AccountCommand).
    # Each signature is 64 bytes + 3 sighash byte = 67 bytes.
    assert len(responses) == 2
    for resp in responses:
        assert len(resp) == TX_RESPONSE_SIZE