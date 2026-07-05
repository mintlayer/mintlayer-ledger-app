from contextlib import contextmanager
from dataclasses import dataclass
from enum import IntEnum
from typing import Generator, List

import scalecodec  # type: ignore
from ragger.backend.interface import RAPDU, BackendInterface
from ragger.navigator import NavInsID
from ragger.navigator.navigation_scenario import NavigationScenarioData, UseCase

from .mintlayer_utils import (
    Transaction,
    TxInputSignatureResponse,
    TxInputSignature,
    decode_response_variant,
    mintlayer_hash,
    sign_tx_start_req_obj,
    sign_tx_next_req_obj,
    verify_tx_signature,
)

MAX_APDU_LEN: int = 255

CLA: int = 0xE1


@dataclass
class ReviewTransaction:
    transaction: Transaction
    has_command_input: bool
    review_custom_screen_text: str


@dataclass
class SignTxStep:
    kind: str


class GetAppAndVersionP1(IntEnum):
    # Parameter 1 for first APDU number.
    P1_START = 0x00
    P1_NEXT = 0x01


class SignTxP1(IntEnum):
    # Parameter 1 for first APDU number.
    P1_START = 0x00
    P1_NEXT = 0x01


class SignMessageP1(IntEnum):
    # Parameter 1 for first APDU number.
    P1_START = 0x00
    P1_NEXT = 0x01


class GetPublicKeyP1(IntEnum):
    P1_DO_NOT_CONFIRM = 0x00
    # Parameter 1 for screen confirmation for GET_PUBLIC_KEY.
    P1_CONFIRM = 0x01


class P2(IntEnum):
    # Parameter 2 for last APDU to receive.
    P2_LAST = 0x00
    # Parameter 2 for more APDU to receive.
    P2_MORE = 0x80


class InsType(IntEnum):
    GET_PUBLIC_KEY = 0x00
    SIGN_TX = 0x01
    SIGN_MESSAGE = 0x02
    PING = 0x03


class Errors(IntEnum):
    SW_DENY = 0x6985
    SW_CLA_NOT_SUPPORTED = 0x6E00
    SW_INS_NOT_SUPPORTED = 0x6E01
    SW_WRONG_P1P2 = 0x6E02
    SW_WRONG_APDU_LENGTH = 0x6E03

    SW_TX_DISPLAY_FAIL = 0xB000
    SW_TX_LOCK_TIME_INVALID = 0xB001
    SW_WRONG_TX_LENGTH = 0xB002
    SW_WRONG_CONTEXT = 0xB008
    SW_DESERIALIZE_FAIL = 0xB009
    SW_MAX_BUFFER_LEN_EXCEEDED = 0xB012


def split_message(message: bytes, max_size: int) -> List[bytes]:
    if len(message) == 0:
        return [b""]

    return [message[x : x + max_size] for x in range(0, len(message), max_size)]


class MintlayerCommandSender:
    def __init__(self, backend: BackendInterface) -> None:
        self.backend = backend

    def get_app_and_version(self) -> RAPDU:
        return self.backend.exchange(
            cla=0xB0,  # specific CLA for BOLOS
            ins=0x01,  # specific INS for get_app_and_version
            p1=GetAppAndVersionP1.P1_START,
            p2=P2.P2_LAST,
            data=b"",
        )

    def get_public_key(self, coin_type: int, path: list[int]) -> RAPDU:
        data = coin_type.to_bytes(1, "little") + pack_derivation_path(path)

        return self.backend.exchange(
            cla=CLA,
            ins=InsType.GET_PUBLIC_KEY,
            p1=GetPublicKeyP1.P1_DO_NOT_CONFIRM,
            p2=P2.P2_LAST,
            data=data,
        )

    @contextmanager
    def get_public_key_with_confirmation(
        self, coin_type: int, path: list[int]
    ) -> Generator[None, None, None]:
        data = coin_type.to_bytes(1, "little") + pack_derivation_path(path)

        with self.backend.exchange_async(
            cla=CLA,
            ins=InsType.GET_PUBLIC_KEY,
            p1=GetPublicKeyP1.P1_CONFIRM,
            p2=P2.P2_LAST,
            data=data,
        ):
            yield

    @contextmanager
    def sign_message(
        self, coin_type: int, addr_type: int, path: list[int], message: bytes
    ) -> Generator[None, None, None]:
        data = (
            coin_type.to_bytes(1, "little")
            + addr_type.to_bytes(1, "little")
            + pack_derivation_path(path)
        )

        response = self.backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_MESSAGE,
            p1=SignMessageP1.P1_START,
            p2=P2.P2_LAST,
            data=data,
        )
        decode_response_variant(response.data, "MessageSetup")

        chunks = split_message(message, MAX_APDU_LEN)

        for chunk in chunks[:-1]:
            response = self.backend.exchange(
                cla=CLA,
                ins=InsType.SIGN_MESSAGE,
                p1=SignMessageP1.P1_NEXT,
                p2=P2.P2_MORE,
                data=chunk,
            )
            decode_response_variant(response.data, "ExpectingNextChunk")

        with self.backend.exchange_async(
            cla=CLA,
            ins=InsType.SIGN_MESSAGE,
            p1=SignMessageP1.P1_NEXT,
            p2=P2.P2_LAST,
            data=chunks[-1],
        ):
            yield

    # pylint: disable-next=too-many-locals
    def sign_tx(self, transaction: Transaction) -> Generator[SignTxStep, None, None]:
        # ---- Start req ----
        start_req = sign_tx_start_req_obj.encode(
            {
                "coin_type": transaction.coin_type,
                "version": 0,
                "num_inputs": len(transaction.inputs),
                "num_outputs": len(transaction.outputs),
            }
        ).data

        response = self.backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_TX,
            p1=SignTxP1.P1_START,
            p2=P2.P2_LAST,
            data=bytes(start_req),
        )
        decode_response_variant(response.data, "TxSetup")

        # ---- INPUTS ----
        print("sending inputs", len(transaction.inputs))

        def encode_input(inp):
            return sign_tx_next_req_obj.encode({"ProcessInput": inp}).data

        def encode_input_comm(comm):
            return sign_tx_next_req_obj.encode({"ProcessInputCommitment": comm}).data

        def encode_output(outp):
            return sign_tx_next_req_obj.encode({"ProcessOutput": outp}).data

        for inp in transaction.inputs:
            encoded_inp = encode_input(inp)
            self._send_chunked_sync(encoded_inp, "TxNext")

        # ---- INPUT COMMITMENTS ----
        print("sending input commitments")

        for comm in transaction.input_commitments[:-1]:
            encoded_comm = encode_input_comm(comm)
            self._send_chunked_sync(encoded_comm, "TxNext")

        encoded_comm = encode_input_comm(transaction.input_commitments[-1])
        chunks = split_message(encoded_comm, MAX_APDU_LEN)

        # all but last chunk sync
        for chunk in chunks[:-1]:
            response = self.backend.exchange(
                cla=CLA,
                ins=InsType.SIGN_TX,
                p1=SignTxP1.P1_NEXT,
                p2=P2.P2_MORE,
                data=chunk,
            )
            decode_response_variant(response.data, "ExpectingNextChunk")

        # last chunk async -> UI review
        with self.backend.exchange_async(
            cla=CLA,
            ins=InsType.SIGN_TX,
            p1=SignTxP1.P1_NEXT,
            p2=P2.P2_LAST,
            data=chunks[-1],
        ):
            yield SignTxStep(kind="start")

            if len(transaction.outputs) == 0:
                yield SignTxStep(kind="sign")

        response = self.get_async_response()
        decode_response_variant(response.data, "TxNext")

        # ---- OUTPUTS ----
        print("streaming outputs")

        for idx, out in enumerate(transaction.outputs):
            print(f"sending output {idx}")

            encoded_out = encode_output(out)
            chunks = split_message(encoded_out, MAX_APDU_LEN)

            # all but last chunk sync
            for chunk in chunks[:-1]:
                response = self.backend.exchange(
                    cla=CLA,
                    ins=InsType.SIGN_TX,
                    p1=SignTxP1.P1_NEXT,
                    p2=P2.P2_MORE,
                    data=chunk,
                )
                decode_response_variant(response.data, "ExpectingNextChunk")

            # last chunk async -> UI review
            with self.backend.exchange_async(
                cla=CLA,
                ins=InsType.SIGN_TX,
                p1=SignTxP1.P1_NEXT,
                p2=P2.P2_LAST,
                data=chunks[-1],
            ):
                kind = "sign" if idx == len(transaction.outputs) - 1 else "output"
                yield SignTxStep(kind=kind)

            response = self.get_async_response()
            decode_response_variant(response.data, "TxNext")

    def _send_chunked_sync(self, data: bytes, expected_last_response_variant: str):
        chunks = split_message(data, MAX_APDU_LEN)

        for chunk in chunks[:-1]:
            response = self.backend.exchange(
                cla=CLA,
                ins=InsType.SIGN_TX,
                p1=SignTxP1.P1_NEXT,
                p2=P2.P2_MORE,
                data=chunk,
            )
            decode_response_variant(response.data, "ExpectingNextChunk")

        response = self.backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_TX,
            p1=SignTxP1.P1_NEXT,
            p2=P2.P2_LAST,
            data=chunks[-1],
        )
        return decode_response_variant(response.data, expected_last_response_variant)

    def get_async_response(self) -> RAPDU:
        response = self.backend.last_async_response
        assert response is not None
        return response

    def get_all_signatures(self, transaction: Transaction) -> List[TxInputSignature]:
        next_sig = sign_tx_next_req_obj.encode({"ReturnNextSignature": None}).data
        sigs = []
        expected_sigs_count = len(transaction.expected_sig_indices())

        while True:
            res = self.backend.exchange(
                cla=CLA,
                ins=InsType.SIGN_TX,
                p1=SignTxP1.P1_NEXT,
                p2=P2.P2_LAST,
                data=next_sig,
            )
            res = TxInputSignatureResponse.from_data(res.data)

            sigs.append(TxInputSignature.from_response(res))

            if not res.has_next:
                break

            assert len(sigs) < expected_sigs_count, (
                "has_next is still true after the expected number of signatures "
                f"have been received (sigs = {sigs!r})"
            )
        return sigs


def pack_derivation_path(path: list[int]) -> bytes:
    path_obj = scalecodec.base.RuntimeConfiguration().create_scale_object("Bip32Path")
    return path_obj.encode(path).data


def _compress_public_key(uncompressed_public_key: bytes) -> bytes:
    assert len(uncompressed_public_key) == 65
    assert uncompressed_public_key[0] == 0x04

    prefix = 0x02 if uncompressed_public_key[64] % 2 == 0 else 0x03
    return bytes([prefix]) + uncompressed_public_key[1:33]


def _public_key_destination(public_key: bytes) -> dict:
    compressed_public_key = _compress_public_key(public_key)
    return {
        "PublicKey": {
            "key": {
                "Secp256k1Schnorr": {
                    "pubkey_data": compressed_public_key,
                }
            }
        }
    }


def _public_key_hash_destination(public_key: bytes) -> dict:
    compressed_public_key = _compress_public_key(public_key)
    encoded_public_key = bytes([0]) + compressed_public_key
    return {"PublicKeyHash": mintlayer_hash(encoded_public_key)[:20]}


def fetch_public_key(
    client: MintlayerCommandSender, coin_type: int, path: list[int]
) -> bytes:
    rapdu = client.get_public_key(coin_type, path)
    msg = decode_response_variant(rapdu.data, "PublicKey")

    public_key = bytes.fromhex(msg["public_key"][2:])
    assert len(public_key) == 65

    chain_code = bytes.fromhex(msg["chain_code"][2:])
    assert len(chain_code) == 32

    return public_key


def fetch_public_key_as_pk_destination(
    client: MintlayerCommandSender, coin_type: int, path: list[int]
) -> dict:
    return _public_key_destination(fetch_public_key(client, coin_type, path))


def fetch_public_key_as_pkh_destination(
    client: MintlayerCommandSender, coin_type: int, path: list[int]
) -> dict:
    return _public_key_hash_destination(fetch_public_key(client, coin_type, path))


# pylint: disable-next=too-many-locals,too-many-branches,too-many-statements
def sign_tx_review(
    client,
    device,
    navigator,
    scenario_navigator,
    review_transaction: ReviewTransaction,
):
    transaction = review_transaction.transaction
    has_command_input = review_transaction.has_command_input
    review_custom_screen_text = review_transaction.review_custom_screen_text

    addr_paths_by_indices = transaction.addr_paths_by_indices()
    pubkeys_by_indices = {}
    for indices, addr_path in addr_paths_by_indices.items():
        pubkeys_by_indices[indices] = fetch_public_key(
            client, transaction.coin_type, addr_path
        )

    # The snapshot index (used to make its name) and the amount by which it should be increased
    # after each step. The increase should be large enough, so that snapshots from later steps
    # don't overwrite the previous ones (10 is not enough).
    start_idx = 0
    idx_inc = 100

    if not device.is_nano:
        instruction = NavInsID.SWIPE_CENTER_TO_LEFT
    else:
        instruction = NavInsID.RIGHT_CLICK

    last_page_pattern = r".*\((\d+)/\1\)$"

    for step in client.sign_tx(transaction):
        print("step kind: ", step.kind)
        if step.kind == "start":
            navigator.navigate_and_compare(
                path=scenario_navigator.screenshot_path,
                test_case_name=scenario_navigator.test_name,
                instructions=[instruction],
                screen_change_before_first_instruction=False,
                screen_change_after_last_instruction=False,
                snap_start_idx=start_idx,
            )
            start_idx += idx_inc

            if has_command_input:
                if device.is_nano:
                    navigator.navigate_until_text_and_compare(
                        navigate_instruction=instruction,
                        validation_instructions=[instruction],
                        text=last_page_pattern,
                        path=scenario_navigator.screenshot_path,
                        test_case_name=scenario_navigator.test_name,
                        screen_change_before_first_instruction=False,
                        screen_change_after_last_instruction=False,
                        snap_start_idx=start_idx,
                    )
                else:
                    navigator.navigate_and_compare(
                        path=scenario_navigator.screenshot_path,
                        test_case_name=scenario_navigator.test_name,
                        instructions=[instruction] * 2,
                        screen_change_before_first_instruction=False,
                        screen_change_after_last_instruction=False,
                        snap_start_idx=start_idx,
                    )
                start_idx += idx_inc

        if step.kind == "output":
            if device.is_nano:
                navigator.navigate_until_text_and_compare(
                    navigate_instruction=instruction,
                    validation_instructions=[instruction],
                    text=last_page_pattern,
                    path=scenario_navigator.screenshot_path,
                    test_case_name=scenario_navigator.test_name,
                    screen_change_before_first_instruction=False,
                    screen_change_after_last_instruction=False,
                    snap_start_idx=start_idx,
                )
            else:
                navigator.navigate_and_compare(
                    path=scenario_navigator.screenshot_path,
                    test_case_name=scenario_navigator.test_name,
                    instructions=[instruction] * 2,
                    screen_change_before_first_instruction=False,
                    screen_change_after_last_instruction=False,
                    snap_start_idx=start_idx,
                )
            start_idx += idx_inc

        elif step.kind == "sign":
            scenario = NavigationScenarioData(
                scenario_navigator.device,
                scenario_navigator.backend,
                UseCase.TX_REVIEW,
                True,
            )

            if device.is_nano:
                validation_instructions = scenario.validation
            else:
                # On touch devices `UseCase.TX_REVIEW` sets `scenario.validation` to
                # `[USE_CASE_REVIEW_CONFIRM, USE_CASE_STATUS_DISMISS]`. But the status screen
                # appears only after the last `ReturnNextSignature`.
                validation_instructions = [NavInsID.USE_CASE_REVIEW_CONFIRM]

            navigator.navigate_until_text_and_compare(
                navigate_instruction=scenario.navigation,
                validation_instructions=validation_instructions,
                text=review_custom_screen_text,
                path=scenario_navigator.screenshot_path,
                test_case_name=scenario_navigator.test_name,
                screen_change_after_last_instruction=False,
                snap_start_idx=start_idx,
            )
            start_idx += idx_inc

    # After review approval, explicitly request every signature.
    signatures = client.get_all_signatures(transaction)

    # The last ReturnNextSignature is what makes the tx Finished, so the "Transaction signed"
    # status screen is expected here.
    validation_instructions = (
        [] if device.is_nano else [NavInsID.USE_CASE_STATUS_DISMISS]
    )
    navigator.navigate_until_text_and_compare(
        navigate_instruction=NavInsID.WAIT_FOR_SCREEN_CHANGE,
        validation_instructions=validation_instructions,
        text=r"^Transaction signed$",
        path=scenario_navigator.screenshot_path,
        test_case_name=scenario_navigator.test_name,
        screen_change_before_first_instruction=False,
        screen_change_after_last_instruction=False,
        snap_start_idx=start_idx,
    )

    sig_indices = {sig.indices() for sig in signatures}
    expected_sig_indices = transaction.expected_sig_indices()
    assert (
        sig_indices == expected_sig_indices
    ), f"Sig indices don't match, expected: {expected_sig_indices}, actual: {sig_indices}"

    addr_paths_by_indices = transaction.addr_paths_by_indices()

    for sig in signatures:
        pubkey = pubkeys_by_indices[sig.indices()]
        sig_valid = verify_tx_signature(transaction, pubkey, sig.signature)
        assert sig_valid, f"Signature verification failed for {sig.indices()}"
