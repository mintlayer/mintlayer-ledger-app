import pytest

from application_client.boilerplate_command_sender import BoilerplateCommandSender, Errors
from application_client.boilerplate_response_unpacker import unpack_sign_message_response
from application_client import MAINNET
from ragger.bip import calculate_public_key_and_chaincode, CurveChoice
from ragger.error import ExceptionRAPDU
from ragger.navigator import NavInsID, NavIns
from utils import ROOT_SCREENSHOT_PATH

MNEMONIC = ("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")

# In this test we check that the message signing works
def test_sign_message(backend, scenario_navigator):
    path = "m/44'/19788'/0'/0/0"
    message = b"Hello"
    client = BoilerplateCommandSender(backend)
    with client.sign_message(coin=MAINNET, addr_type=0, path=path, message=message):
        scenario_navigator.review_approve()

    response = client.get_async_response().data
    _, sig = unpack_sign_message_response(response)

def test_sign_message_pkh(backend, scenario_navigator):
    path = "m/44'/19788'/0'/0/0"
    message = b"Hello"
    client = BoilerplateCommandSender(backend)
    with client.sign_message(coin=MAINNET, addr_type=1, path=path, message=message):
        scenario_navigator.review_approve()

    response = client.get_async_response().data
    _, sig = unpack_sign_message_response(response)

# Message signing refused test
# The test will ask for a message signature that will be refused on screen
def test_sign_message_refused(backend, scenario_navigator):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    path: str = "m/44'/19788'/0'/0/0"
    message: bytes = b"Hello"

    with pytest.raises(ExceptionRAPDU) as e:
        with client.sign_message(coin=MAINNET, addr_type=0, path=path, message=message):
            scenario_navigator.review_reject()
    
    # Assert that we have received a refusal
    assert e.value.status == Errors.SW_DENY
    assert len(e.value.data) == 0