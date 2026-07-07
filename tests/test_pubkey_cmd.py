import pytest
from ragger.bip import CurveChoice, calculate_public_key_and_chaincode
from ragger.error import ExceptionRAPDU

from application_client import MAINNET, TESTNET
from application_client.mintlayer_command_sender import Errors, MintlayerCommandSender
from application_client.mintlayer_response_unpacker import (
    unpack_get_public_key_response,
)
from application_client.mintlayer_utils import parse_derivation_path

MNEMONIC = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"


# In this test we check that the GET_PUBLIC_KEY works in non-confirmation mode
def test_get_public_key_no_confirm(backend):
    for path in [
        "m/44'/19788'/0'/0/0",
        "m/44'/19788'/911'/0/0",
        "m/44'/19788'/255'/255/255",
        "m/44'/19788'/2147483647'/0/0/0/0/0/0/0",
    ]:
        client = MintlayerCommandSender(backend)
        response = client.get_public_key(MAINNET, parse_derivation_path(path)).data
        _, public_key, _, _ = unpack_get_public_key_response(response)

        ref_public_key, _ = calculate_public_key_and_chaincode(
            CurveChoice.Secp256k1, path=path, mnemonic=MNEMONIC
        )
        assert public_key.hex() == ref_public_key


def test_get_public_key_no_confirm_testnet(backend):
    for path in [
        "m/44'/1'/0'/0/0",
        "m/44'/1'/911'/0/0",
        "m/44'/1'/255'/255/255",
        "m/44'/1'/2147483647'/0/0/0/0/0/0/0",
    ]:
        client = MintlayerCommandSender(backend)
        response = client.get_public_key(TESTNET, parse_derivation_path(path)).data
        _, public_key, _, _ = unpack_get_public_key_response(response)

        ref_public_key, _ = calculate_public_key_and_chaincode(
            CurveChoice.Secp256k1, path=path, mnemonic=MNEMONIC
        )
        assert public_key.hex() == ref_public_key


def test_get_public_key_non_hardened_account_index(backend):
    client = MintlayerCommandSender(backend)
    path = "m/44'/19788'/0/0/0"

    with pytest.raises(ExceptionRAPDU) as e:
        client.get_public_key(MAINNET, parse_derivation_path(path))

    assert e.value.status == Errors.SW_INVALID_PATH
    assert len(e.value.data) == 0


# In this test we check that the GET_PUBLIC_KEY works in confirmation mode
def test_get_public_key_confirm_accepted(backend, scenario_navigator):
    client = MintlayerCommandSender(backend)
    path = "m/44'/19788'/0'/0/0"

    with client.get_public_key_with_confirmation(
        MAINNET, parse_derivation_path(path)
    ):
        scenario_navigator.address_review_approve()

    response = client.get_async_response().data
    _, public_key, _, _ = unpack_get_public_key_response(response)

    ref_public_key, _ = calculate_public_key_and_chaincode(
        CurveChoice.Secp256k1, path=path, mnemonic=MNEMONIC
    )
    assert public_key.hex() == ref_public_key


# In this test we check that the GET_PUBLIC_KEY in confirmation mode replies an error if the user refuses
def test_get_public_key_confirm_refused(backend, scenario_navigator):
    client = MintlayerCommandSender(backend)
    path = "m/44'/19788'/0'/0/0"

    with pytest.raises(ExceptionRAPDU) as e:
        with client.get_public_key_with_confirmation(
            MAINNET, parse_derivation_path(path)
        ):
            scenario_navigator.address_review_reject()

    # Assert that we have received a refusal
    assert e.value.status == Errors.SW_DENY
    assert len(e.value.data) == 0
