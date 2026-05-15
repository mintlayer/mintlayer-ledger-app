import pytest
import scalecodec  # type: ignore
from ragger.error import ExceptionRAPDU

from application_client import MAINNET
from application_client.mintlayer_command_sender import (
    Errors, MintlayerCommandSender)


# In this test we check that the message signing works
def test_sign_message(backend, scenario_navigator):
    path = "m/44'/19788'/0'/0/0"
    message = b"Hello"
    client = MintlayerCommandSender(backend)
    with client.sign_message(coin=MAINNET, addr_type=0, path=path, message=message):
        scenario_navigator.review_approve()

    response = scalecodec.base.ScaleBytes(client.get_async_response().data)
    msg_signature_obj = scalecodec.base.RuntimeConfiguration().create_scale_object("MsgSignature", data=response)
    sig = msg_signature_obj.decode()

def test_sign_message_pkh(backend, scenario_navigator):
    path = "m/44'/19788'/0'/0/0"
    message = b"Hello"
    client = MintlayerCommandSender(backend)
    with client.sign_message(coin=MAINNET, addr_type=1, path=path, message=message):
        scenario_navigator.review_approve()

    response = scalecodec.base.ScaleBytes(client.get_async_response().data)
    msg_signature_obj = scalecodec.base.RuntimeConfiguration().create_scale_object("MsgSignature", data=response)
    sig = msg_signature_obj.decode()


# Message signing refused test
# The test will ask for a message signature that will be refused on screen
def test_sign_message_refused(backend, scenario_navigator):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    path: str = "m/44'/19788'/0'/0/0"
    message: bytes = b"Hello"

    with pytest.raises(ExceptionRAPDU) as e:
        with client.sign_message(coin=MAINNET, addr_type=0, path=path, message=message):
            scenario_navigator.review_reject()

    # Assert that we have received a refusal
    assert e.value.status == Errors.SW_DENY
    assert len(e.value.data) == 0
