import scalecodec  # type: ignore
from dataclasses import dataclass
from typing import List, Optional

sign_tx_start_req_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
    "SignTxStartReq"
)
sign_tx_next_req_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
    "SignTxNextReq"
)


class TransactionError(Exception):
    pass


@dataclass
class TxInputSignatureResponse:
    signature: bytes
    input_idx: int
    multisig_idx: Optional[int]
    has_next: bool

    @staticmethod
    def from_data(response: bytes):
        response = decode_response_variant(response, "TxInputSignature")

        signature = bytes.fromhex(response["signature"][2:])
        assert len(signature) == 64

        return TxInputSignatureResponse(
            signature=signature,
            input_idx=response["input_idx"],
            multisig_idx=response["multisig_idx"],
            has_next=response["has_next"],
        )


@dataclass(frozen=True)
class TxInputSignatureIndices:
    input_idx: int
    multisig_idx: Optional[int]


@dataclass
class TxInputSignature:
    signature: bytes
    input_idx: int
    multisig_idx: Optional[int]

    @staticmethod
    def from_response(response: TxInputSignatureResponse):
        return TxInputSignature(
            signature=response.signature,
            input_idx=response.input_idx,
            multisig_idx=response.multisig_idx,
        )

    def indices(self) -> TxInputSignatureIndices:
        return TxInputSignatureIndices(
            input_idx=self.input_idx,
            multisig_idx=self.multisig_idx,
        )


@dataclass
class Transaction:
    coin: int
    inputs: List[dict]
    input_commitments: List[dict]
    outputs: List[dict]

    def expected_sig_indices(self) -> set[TxInputSignatureIndices]:
        result = set()
        for input_idx, input in enumerate(self.inputs):
            input_data = input.get("ProcessInput")
            assert (
                input_data is not None
            ), f"Transaction input is not a ProcessInput request: {input!r}"

            for addr in input_data["addresses"]:
                multisig_idx = addr["multisig_idx"]
                result.add(
                    TxInputSignatureIndices(
                        input_idx=input_idx, multisig_idx=multisig_idx
                    )
                )

        return result


def decode_response(response: bytes):
    response_bytes = scalecodec.base.ScaleBytes(response)
    response_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
        "Response", data=response_bytes
    )
    return response_obj.decode()


def decode_response_variant(response: bytes, expected_variant: str):
    response = decode_response(response)

    assert (
        isinstance(response, dict)
        and len(response) == 1
        and response[expected_variant] is not None
    ), f"Expecting a dict with a single key '{expected_variant}', but got: {response!r}"

    return response[expected_variant]
