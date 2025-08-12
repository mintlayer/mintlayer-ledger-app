import pytest

from ragger.error import ExceptionRAPDU
from application_client.boilerplate_command_sender import CLA, InsType, P1, P2, Errors
from application_client import MAINNET


# Ensure the app returns an error when a bad CLA is used
def test_bad_cla(backend):
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA + 1, ins=InsType.GET_VERSION)
    assert e.value.status == Errors.SW_CLA_NOT_SUPPORTED


# Ensure the app returns an error when a bad INS is used
def test_bad_ins(backend):
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=0xff)
    assert e.value.status == Errors.SW_INS_NOT_SUPPORTED


# Ensure the app returns an error when a bad P1 or P2 is used
def test_wrong_p1p2(backend):
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.GET_VERSION, p1=P1.P1_START + 1, p2=P2.P2_LAST)
    assert e.value.status == Errors.SW_WRONG_P1P2
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.GET_VERSION, p1=P1.P1_START, p2=P2.P2_MORE)
    assert e.value.status == Errors.SW_WRONG_P1P2
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.GET_APP_NAME, p1=P1.P1_START + 1, p2=P2.P2_LAST)
    assert e.value.status == Errors.SW_WRONG_P1P2
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.GET_APP_NAME, p1=P1.P1_START, p2=P2.P2_MORE)
    assert e.value.status == Errors.SW_WRONG_P1P2

# Ensure the app returns an error when a bad data length is used
def test_wrong_data_length(backend):
    # APDUs must be at least 4 bytes: CLA, INS, P1, P2.
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange_raw(bytes.fromhex("E00300"))
    assert e.value.status == Errors.SW_WRONG_APDU_LENGTH
    # APDUs advertises a too long length
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange_raw(bytes.fromhex("E003000005"))
    assert e.value.status == Errors.SW_WRONG_APDU_LENGTH


# Ensure there is no state confusion when trying wrong APDU sequences
def test_invalid_state(backend):
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA,
                         ins=InsType.SIGN_TX,
                         p1=P1.P1_TX_INPUT,  # Try to continue a flow instead of start a new one
                         p2=P2.P2_MORE)
    assert e.value.status == Errors.SW_WRONG_CONTEXT


def test_sign_tx_invalid_coin(backend, scenario_navigator, device, navigator):
    invalid_coin = 255
    num_inputs = 1
    num_outputs = 1
    metadata = bytes([
        #1 + 1 + 4 + 4, # len
        invalid_coin,
        1, # version
        ]) + num_inputs.to_bytes(byteorder="big", length=4) + num_outputs.to_bytes(byteorder="big", length=4)

    with pytest.raises(ExceptionRAPDU) as e:
        res = backend.exchange(cla=CLA,
                            ins=InsType.SIGN_TX,
                            p1=P1.P1_START,
                            p2=P2.P2_MORE,
                            data=bytes(metadata))
        
    assert e.value.status == Errors.SW_DESERIALIZE_FAIL

def test_sign_tx_invalid_P2_for_input(backend, scenario_navigator, device, navigator):
    """
    After metadata try to pass input commitment instead of the input
    expect an error for wrong P1/P2
    """
    num_inputs = 2
    num_outputs = 2
    metadata = bytes([
        #1 + 1 + 4 + 4, # len
        MAINNET,
        1, # version
        ]) + num_inputs.to_bytes(byteorder="big", length=4) + num_outputs.to_bytes(byteorder="big", length=4)

    res = backend.exchange(cla=CLA,
                        ins=InsType.SIGN_TX,
                        p1=P1.P1_START,
                        p2=P2.P2_MORE,
                        data=bytes(metadata))
    
    assert res.status == 0x9000

    with pytest.raises(ExceptionRAPDU) as e:
        res = backend.exchange(cla=CLA,
                            ins=InsType.SIGN_TX,
                            p1=P1.P1_TX_INPUT_COMMITMENT,
                            p2=P2.P2_LAST,
                            data=b"")
        
    assert e.value.status == Errors.SW_WRONG_P1P2

def test_sign_tx_invalid_input(backend, scenario_navigator, device, navigator):
    num_inputs = 2
    num_outputs = 2
    metadata = bytes([
        #1 + 1 + 4 + 4, # len
        MAINNET,
        1, # version
        ]) + num_inputs.to_bytes(byteorder="big", length=4) + num_outputs.to_bytes(byteorder="big", length=4)

    res = backend.exchange(cla=CLA,
                        ins=InsType.SIGN_TX,
                        p1=P1.P1_START,
                        p2=P2.P2_MORE,
                        data=bytes(metadata))
    
    print("res, ", res.status)
    assert res.status == 0x9000

    with pytest.raises(ExceptionRAPDU) as e:
        res = backend.exchange(cla=CLA,
                            ins=InsType.SIGN_TX,
                            p1=P1.P1_TX_INPUT,
                            p2=P2.P2_LAST,
                            data=bytes([0]*10))
        
    assert e.value.status == Errors.SW_DESERIALIZE_FAIL


def test_sign_tx_too_large_data(backend, scenario_navigator, device, navigator):
    num_inputs = 2
    num_outputs = 2
    metadata = bytes([
        #1 + 1 + 4 + 4, # len
        MAINNET,
        1, # version
        ]) + num_inputs.to_bytes(byteorder="big", length=4) + num_outputs.to_bytes(byteorder="big", length=4)

    res = backend.exchange(cla=CLA,
                        ins=InsType.SIGN_TX,
                        p1=P1.P1_START,
                        p2=P2.P2_MORE,
                        data=bytes(metadata))
    
    assert res.status == 0x9000

    with pytest.raises(ExceptionRAPDU) as e:
        for _ in range(100):
            res = backend.exchange(cla=CLA,
                                ins=InsType.SIGN_TX,
                                p1=P1.P1_TX_INPUT,
                                p2=P2.P2_MORE,
                                data=b"big_input")
        
    assert e.value.status == Errors.SW_WRONG_TX_LENGTH