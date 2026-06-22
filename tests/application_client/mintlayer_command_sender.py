from contextlib import contextmanager
from dataclasses import dataclass
from enum import IntEnum
from typing import Any, Generator, List, Optional

import scalecodec  # type: ignore
from ragger.backend.interface import RAPDU, BackendInterface
from ragger.navigator import NavInsID
from ragger.navigator.navigation_scenario import NavigationScenarioData, UseCase

from .mintlayer_transaction import Transaction

sign_tx_start_req_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
    "SignTxStartReq"
)
sign_tx_next_req_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
    "SignTxNextReq"
)

MAX_APDU_LEN: int = 255
TX_RESPONSE_SIZE: int = 71

CLA: int = 0xE1


@dataclass
class ReviewTransaction:
    transaction: Transaction
    has_command_input: bool
    review_custom_screen_text: str


@dataclass
class SignTxStep:
    kind: str
    index: int | None = None


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

        self.backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_MESSAGE,
            p1=SignMessageP1.P1_START,
            p2=P2.P2_LAST,
            data=data,
        )
        chunks = split_message(message, MAX_APDU_LEN)

        for chunk in chunks[:-1]:
            self.backend.exchange(
                cla=CLA,
                ins=InsType.SIGN_MESSAGE,
                p1=SignMessageP1.P1_NEXT,
                p2=P2.P2_MORE,
                data=chunk,
            )

        with self.backend.exchange_async(
            cla=CLA,
            ins=InsType.SIGN_MESSAGE,
            p1=SignMessageP1.P1_NEXT,
            p2=P2.P2_LAST,
            data=chunks[-1],
        ) as response:
            yield response

    def sign_tx(self, transaction: Transaction) -> Generator[SignTxStep, None, None]:
        # ---- METADATA ----
        start_req = sign_tx_start_req_obj.encode(
            {
                "coin": transaction.coin,
                "version": 0,
                "num_inputs": len(transaction.inputs),
                "num_outputs": len(transaction.outputs),
            }
        ).data

        res = self.backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_TX,
            p1=SignTxP1.P1_START,
            p2=P2.P2_LAST,
            data=bytes(start_req),
        )
        print("metadata ", res)

        # ---- INPUTS ----
        print("sending inputs", len(transaction.inputs))

        for inp in transaction.inputs:
            self._send_chunked_sync(inp)

        # ---- INPUT COMMITMENTS ----
        print("sending input commitments")

        for inp in transaction.input_commitments[:-1]:
            self._send_chunked_sync(inp)

        chunks = split_message(transaction.input_commitments[-1], MAX_APDU_LEN)

        # all but last chunk sync
        for chunk in chunks[:-1]:
            self.backend.exchange(
                cla=CLA,
                ins=InsType.SIGN_TX,
                p1=SignTxP1.P1_NEXT,
                p2=P2.P2_MORE,
                data=chunk,
            )

        # last chunk async -> UI review
        with self.backend.exchange_async(
            cla=CLA,
            ins=InsType.SIGN_TX,
            p1=SignTxP1.P1_NEXT,
            p2=P2.P2_LAST,
            data=chunks[-1],
        ):
            kind = "start"
            yield SignTxStep(kind=kind, index=0)

        # ---- OUTPUTS ----
        print("streaming outputs")

        for idx, out in enumerate(transaction.outputs):
            print(f"sending output {idx}")

            chunks = split_message(out, MAX_APDU_LEN)

            # all but last chunk sync
            for chunk in chunks[:-1]:
                self.backend.exchange(
                    cla=CLA,
                    ins=InsType.SIGN_TX,
                    p1=SignTxP1.P1_NEXT,
                    p2=P2.P2_MORE,
                    data=chunk,
                )

            # last chunk async -> UI review
            with self.backend.exchange_async(
                cla=CLA,
                ins=InsType.SIGN_TX,
                p1=SignTxP1.P1_NEXT,
                p2=P2.P2_LAST,
                data=chunks[-1],
            ):
                kind = "final" if idx == len(transaction.outputs) - 1 else "output"
                yield SignTxStep(kind=kind, index=idx)

    def _send_chunked_sync(self, data: bytes):
        chunks = split_message(data, MAX_APDU_LEN)

        for chunk in chunks[:-1]:
            self.backend.exchange(
                cla=CLA,
                ins=InsType.SIGN_TX,
                p1=SignTxP1.P1_NEXT,
                p2=P2.P2_MORE,
                data=chunk,
            )

        self.backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_TX,
            p1=SignTxP1.P1_NEXT,
            p2=P2.P2_LAST,
            data=chunks[-1],
        )

    def get_async_response(self) -> Optional[RAPDU]:
        return self.backend.last_async_response

    def get_all_signatures(self, tx: Transaction) -> List[bytes | Any]:
        if self.backend.last_async_response is None:
            raise ValueError("None response")

        next_sig = sign_tx_next_req_obj.encode({"ReturnNextSignature": None}).data
        responses = [self.backend.last_async_response.data]
        for _ in tx.inputs[1:]:
            res = self.backend.exchange(
                cla=CLA,
                ins=InsType.SIGN_TX,
                p1=SignTxP1.P1_NEXT,
                p2=P2.P2_LAST,
                data=next_sig,
            )
            if res is not None:
                responses.append(res.data)
            else:
                raise ValueError("None response")
        return responses


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
            navigator.navigate_until_text_and_compare(
                navigate_instruction=scenario.navigation,
                validation_instructions=scenario.validation,
                text=review_custom_screen_text,
                path=scenario_navigator.screenshot_path,
                test_case_name=scenario_navigator.test_name,
                screen_change_after_last_instruction=False,
                snap_start_idx=start_idx,
            )

    # The device has yielded the result, parse it and ensure that the signature is correct
    responses = client.get_all_signatures(transaction)

    assert len(responses) == len(transaction.inputs)
    for response in responses:
        assert len(response) == TX_RESPONSE_SIZE
