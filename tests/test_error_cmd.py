import pytest
from ragger.error import ExceptionRAPDU

from application_client import MAINNET
from application_client.mintlayer_command_sender import (
    CLA,
    GetAppAndVersionP1,
    SignTxP1,
    P2,
    Errors,
    InsType,
)
from application_client.mintlayer_utils import (
    sign_tx_start_req_obj,
    sign_tx_next_req_obj,
)


# Ensure the app returns an error when a bad CLA is used
def test_bad_cla(backend):
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA + 1, ins=InsType.GET_PUBLIC_KEY)
    assert e.value.status == Errors.SW_CLA_NOT_SUPPORTED


# Ensure the app returns an error when a bad INS is used
def test_bad_ins(backend):
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=0xFF)
    assert e.value.status == Errors.SW_INS_NOT_SUPPORTED


# Ensure the app returns an error when a bad P1 or P2 is used
def test_wrong_p1p2(backend):
    # Wrong P2
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(
            cla=CLA, ins=InsType.GET_PUBLIC_KEY, p1=GetAppAndVersionP1.P1_START, p2=123
        )
    assert e.value.status == Errors.SW_WRONG_P1P2

    backend.exchange(
        cla=CLA,
        ins=InsType.GET_PUBLIC_KEY,
        p1=GetAppAndVersionP1.P1_START,
        p2=P2.P2_MORE,
    )

    # Wrong P1 after sending MORE
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(
            cla=CLA,
            ins=InsType.GET_PUBLIC_KEY,
            p1=GetAppAndVersionP1.P1_START + 1,
            p2=P2.P2_MORE,
        )
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
        backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_TX,
            p1=SignTxP1.P1_NEXT,  # Try to continue a flow instead of start a new one
            p2=P2.P2_LAST,
        )
    assert e.value.status == Errors.SW_WRONG_CONTEXT


def test_sign_tx_invalid_coin(backend, scenario_navigator, device, navigator):
    invalid_coin = 255
    num_inputs = 1
    num_outputs = 1
    start_req = sign_tx_start_req_obj.encode(
        {
            "coin_type": invalid_coin,
            "version": 0,
            "num_inputs": num_inputs,
            "num_outputs": num_outputs,
        }
    ).data

    with pytest.raises(ExceptionRAPDU) as e:
        res = backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_TX,
            p1=SignTxP1.P1_START,
            p2=P2.P2_LAST,
            data=bytes(start_req),
        )

    assert e.value.status == Errors.SW_DESERIALIZE_FAIL


def test_sign_tx_invalid_context(backend, scenario_navigator, device, navigator):
    """
    After the start request try to pass an output instead of the input
    expect an error for wrong context
    """
    num_inputs = 2
    num_outputs = 2
    start_req = sign_tx_start_req_obj.encode(
        {
            "coin_type": MAINNET,
            "version": 0,
            "num_inputs": num_inputs,
            "num_outputs": num_outputs,
        }
    ).data

    res = backend.exchange(
        cla=CLA,
        ins=InsType.SIGN_TX,
        p1=SignTxP1.P1_START,
        p2=P2.P2_LAST,
        data=bytes(start_req),
    )

    assert res.status == 0x9000

    with pytest.raises(ExceptionRAPDU) as e:
        res = backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_TX,
            p1=SignTxP1.P1_NEXT,
            p2=P2.P2_LAST,
            data=sign_tx_next_req_obj.encode(
                {
                    "ProcessOutput": {
                        "output": {
                            "Transfer": [
                                {"Coin": 10},
                                {
                                    "PublicKey": {
                                        "key": {
                                            "Secp256k1Schnorr": {
                                                "pubkey_data": bytes([0] * 33)
                                            }
                                        }
                                    }
                                },
                            ],
                        }
                    }
                }
            ).data,
        )
    assert e.value.status == Errors.SW_WRONG_CONTEXT


def test_sign_tx_invalid_input(backend, scenario_navigator, device, navigator):
    num_inputs = 2
    num_outputs = 2
    start_req = sign_tx_start_req_obj.encode(
        {
            "coin_type": MAINNET,
            "version": 0,
            "num_inputs": num_inputs,
            "num_outputs": num_outputs,
        }
    ).data

    res = backend.exchange(
        cla=CLA,
        ins=InsType.SIGN_TX,
        p1=SignTxP1.P1_START,
        p2=P2.P2_LAST,
        data=bytes(start_req),
    )

    print("res, ", res.status)
    assert res.status == 0x9000

    with pytest.raises(ExceptionRAPDU) as e:
        res = backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_TX,
            p1=SignTxP1.P1_NEXT,
            p2=P2.P2_LAST,
            data=bytes([0] * 10),
        )

    assert e.value.status == Errors.SW_DESERIALIZE_FAIL


def test_sign_tx_too_large_data(backend, scenario_navigator, device, navigator):
    num_inputs = 2
    num_outputs = 2
    start_req = sign_tx_start_req_obj.encode(
        {
            "coin_type": MAINNET,
            "version": 0,
            "num_inputs": num_inputs,
            "num_outputs": num_outputs,
        }
    ).data

    res = backend.exchange(
        cla=CLA,
        ins=InsType.SIGN_TX,
        p1=SignTxP1.P1_START,
        p2=P2.P2_LAST,
        data=bytes(start_req),
    )

    assert res.status == 0x9000

    with pytest.raises(ExceptionRAPDU) as e:
        for _ in range(1000):
            res = backend.exchange(
                cla=CLA,
                ins=InsType.SIGN_TX,
                p1=SignTxP1.P1_NEXT,
                p2=P2.P2_MORE,
                data=b"input_data",
            )

    assert e.value.status == Errors.SW_MAX_BUFFER_LEN_EXCEEDED
