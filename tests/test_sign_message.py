import pytest
from ragger.error import ExceptionRAPDU

from application_client import MAINNET
from application_client.mintlayer_command_sender import Errors, MintlayerCommandSender
from application_client.mintlayer_response_unpacker import (
    unpack_sign_message_response,
    unpack_get_public_key_response,
)
from application_client.mintlayer_utils import (
    verify_message_signature,
)


# In this test we check that the message signing works
def test_sign_message(backend, scenario_navigator):
    path = "m/44'/19788'/0'/0/0"
    message = b"Hello"
    coin_type = MAINNET
    client = MintlayerCommandSender(backend)

    pubkey = get_pub_key(client, coin_type=coin_type, path=path)

    with client.sign_message(
        coin_type=coin_type, addr_type=0, path=path, message=message
    ):
        scenario_navigator.review_approve()

    sig = unpack_sign_message_response(client.get_async_response().data)

    sig_valid = verify_message_signature(message, pubkey, sig)
    assert sig_valid, "Signature verification failed"


# Same as test_sign_message, but the message is large enough to require chunking
def test_sign_large_message(backend, scenario_navigator):
    path = "m/44'/19788'/0'/0/0"
    message = b"Hello" * 100
    coin_type = MAINNET
    client = MintlayerCommandSender(backend)

    pubkey = get_pub_key(client, coin_type=coin_type, path=path)

    with client.sign_message(
        coin_type=coin_type, addr_type=0, path=path, message=message
    ):
        scenario_navigator.review_approve()

    sig = unpack_sign_message_response(client.get_async_response().data)
    assert len(sig) == 64

    sig_valid = verify_message_signature(message, pubkey, sig)
    assert sig_valid, "Signature verification failed"


def test_sign_message_pkh(backend, scenario_navigator):
    path = "m/44'/19788'/0'/0/0"
    message = b"Hello"
    coin_type = MAINNET
    client = MintlayerCommandSender(backend)

    pubkey = get_pub_key(client, coin_type=coin_type, path=path)

    with client.sign_message(
        coin_type=coin_type, addr_type=1, path=path, message=message
    ):
        scenario_navigator.review_approve()

    sig = unpack_sign_message_response(client.get_async_response().data)

    sig_valid = verify_message_signature(message, pubkey, sig)
    assert sig_valid, "Signature verification failed"


# Message signing refused test
# The test will ask for a message signature that will be refused on screen
def test_sign_message_refused(backend, scenario_navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    path: str = "m/44'/19788'/0'/0/0"
    message: bytes = b"Hello"
    coin_type = MAINNET

    with pytest.raises(ExceptionRAPDU) as e:
        with client.sign_message(
            coin_type=coin_type, addr_type=0, path=path, message=message
        ):
            scenario_navigator.review_reject()

    # Assert that we have received a refusal
    assert e.value.status == Errors.SW_DENY
    assert len(e.value.data) == 0


def get_pub_key(client: MintlayerCommandSender, coin_type: int, path: str) -> bytes:
    rapdu = client.get_public_key(coin_type, path)
    _, pubkey, _, _ = unpack_get_public_key_response(rapdu.data)
    return pubkey
