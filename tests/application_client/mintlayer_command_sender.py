from contextlib import contextmanager
from dataclasses import dataclass
from enum import IntEnum
from typing import Any, Generator, List, Optional

import scalecodec  # type: ignore
from ragger.backend.interface import RAPDU, BackendInterface
from ragger.navigator import NavInsID
from ragger.navigator.navigation_scenario import NavigationScenarioData, UseCase

from .mintlayer_utils import (
    Transaction,
    TxInputSignatureResponse,
    TxInputSignature,
    decode_response_variant,
    sign_tx_start_req_obj,
    sign_tx_next_req_obj,
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
    P1_START = 0x00
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


class Errors(IntEnum):
    SW_DENY = 0x6985
    SW_CLA_NOT_SUPPORTED = 0x6E00
    SW_INS_NOT_SUPPORTED = 0x6E01
    SW_WRONG_P1P2 = 0x6E02
    SW_WRONG_APDU_LENGTH = 0x6E03

    SW_WRONG_RESPONSE_LENGTH = 0xB000
    SW_DISPLAY_BIP32_PATH_FAIL = 0xB001
    SW_WRONG_TX_LENGTH = 0xB002
    SW_WRONG_CONTEXT = 0xB008
    SW_DESERIALIZE_FAIL = 0xB009
    SW_MAX_BUFFER_LEN_EXCEEDED = 0xB012


def split_message(message: bytes, max_size: int) -> List[bytes]:
    return [message[x: x + max_size] for x in range(0, len(message), max_size)]


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

    def get_public_key(self, coin: int, path: str) -> RAPDU:
        data = coin.to_bytes(1, "little") + pack_derivation_path(path)

        return self.backend.exchange(
            cla=CLA,
            ins=InsType.GET_PUBLIC_KEY,
            p1=GetPublicKeyP1.P1_START,
            p2=P2.P2_LAST,
            data=data,
        )

    @contextmanager
    def get_public_key_with_confirmation(
        self, coin: int, path: str
    ) -> Generator[None, None, None]:
        data = coin.to_bytes(1, "little") + pack_derivation_path(path)

        with self.backend.exchange_async(
            cla=CLA,
            ins=InsType.GET_PUBLIC_KEY,
            p1=GetPublicKeyP1.P1_CONFIRM,
            p2=P2.P2_LAST,
            data=data,
        ) as response:
            yield response

    @contextmanager
    def sign_message(
        self, coin: int, addr_type: int, path: str, message: bytes
    ) -> Generator[None, None, None]:
        data = (
            coin.to_bytes(1, "little")
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
        ) as response:
            yield response

    def sign_tx(self, transaction: Transaction) -> Generator[SignTxStep, None, None]:
        # ---- Start req ----
        start_req = sign_tx_start_req_obj.encode(
            {
                "coin": transaction.coin,
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

        for inp in transaction.inputs:
            encoded_inp = sign_tx_next_req_obj.encode(inp).data
            self._send_chunked_sync(encoded_inp, "TxNext")

        # ---- INPUT COMMITMENTS ----
        print("sending input commitments")

        for comm in transaction.input_commitments[:-1]:
            encoded_comm = sign_tx_next_req_obj.encode(comm).data
            self._send_chunked_sync(encoded_comm, "TxNext")

        encoded_comm = sign_tx_next_req_obj.encode(
            transaction.input_commitments[-1]
        ).data
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
                yield SignTxStep(kind="final")

        response = self.get_async_response()
        decode_response_variant(response.data, "TxNext")

        # ---- OUTPUTS ----
        print("streaming outputs")

        for idx, out in enumerate(transaction.outputs):
            print(f"sending output {idx}")

            encoded_out = sign_tx_next_req_obj.encode(out).data
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
                kind = "final" if idx == len(transaction.outputs) - 1 else "output"
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

    def get_async_response(self) -> Optional[RAPDU]:
        return self.backend.last_async_response

    def get_all_signatures(self) -> List[TxInputSignature]:
        next_sig = sign_tx_next_req_obj.encode({"ReturnNextSignature": None}).data
        sigs = []
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
        return sigs


def hardened_index(index: int) -> int:
    return index | 1 << 31


def pack_derivation_path(derivation_path: str) -> bytes:
    path_obj = scalecodec.base.RuntimeConfiguration().create_scale_object("Bip32Path")

    split = derivation_path.split("/")

    if split[0] != "m":
        raise ValueError("Error master expected")

    path = []
    for value in split[1:]:
        if value == "":
            raise ValueError(f'Error missing value in split list "{split}"')
        if value.endswith("'"):
            path.append(hardened_index(int(value[:-1])))
        else:
            path.append(int(value))

    return path_obj.encode(path).data


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

    # FIXME: instead of making the +=10 jumps in the index, it's better to put snapshots for different
    # phases into different subdirs, e.g. use test_case_name=f"{scenario_navigator.test_name}/start"
    # etc.
    start_idx = 0
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
            start_idx += 10

            # FIXME: the fixed 2-step navigate_and_compare for touch devices that is used below is
            # unreliable. Perhaps we should add a separate field to output review saying something
            # like "Output i/n". This might also make the signing process more clear for the user.
            # Same should be done for inputs review (once multiple inputs review is implemented).

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
                start_idx += 10

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
            start_idx += 10

        elif step.kind == "final":
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
            start_idx += 10

    # After review approval, explicitly request every signature.
    signatures = client.get_all_signatures()

    if not device.is_nano:
        # The last ReturnNextSignature is what makes the tx Finished, so on touch devices
        # the "Transaction signed" status screen is expected here.
        navigator.navigate_and_compare(
            path=scenario_navigator.screenshot_path,
            test_case_name=scenario_navigator.test_name,
            instructions=[NavInsID.USE_CASE_STATUS_DISMISS],
            screen_change_before_first_instruction=True,
            screen_change_after_last_instruction=False,
            snap_start_idx=start_idx,
        )

    sig_indices = {sig.indices() for sig in signatures}
    expected_sig_indices = transaction.expected_sig_indices()
    assert (
        sig_indices == expected_sig_indices
    ), f"Sig indices don't match, expected: {expected_sig_indices}, actual: {sig_indices}"
