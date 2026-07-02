from dataclasses import dataclass
from typing import List, Optional
import scalecodec  # type: ignore

sign_tx_start_req_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
    "SignTxStartReq"
)
sign_tx_next_req_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
    "SignTxNextReq"
)


@dataclass
class TxInputSignatureResponse:
    signature: bytes
    input_idx: int
    multisig_idx: Optional[int]
    has_next: bool

    @staticmethod
    def from_data(response: bytes):
        decoded_response = decode_response_variant(response, "TxInputSignature")

        signature = bytes.fromhex(decoded_response["signature"][2:])
        assert len(signature) == 64

        return TxInputSignatureResponse(
            signature=signature,
            input_idx=decoded_response["input_idx"],
            multisig_idx=decoded_response["multisig_idx"],
            has_next=decoded_response["has_next"],
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
    # A list of TxInputData objects.
    inputs: List[dict]
    # A list of TxInputCommitmentData objects.
    input_commitments: List[dict]
    # A list of TxOutputData objects.
    outputs: List[dict]

    def expected_sig_indices(self) -> set[TxInputSignatureIndices]:
        result = set()
        for input_idx, inp in enumerate(self.inputs):
            for addr in inp["addresses"]:
                multisig_idx = addr["multisig_idx"]
                result.add(
                    TxInputSignatureIndices(
                        input_idx=input_idx, multisig_idx=multisig_idx
                    )
                )

        return result


def decode_response(response: bytes) -> dict:
    response_bytes = scalecodec.base.ScaleBytes(response)
    response_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
        "Response", data=response_bytes
    )
    return response_obj.decode()


def decode_response_variant(response: bytes, expected_variant: str) -> dict:
    decoded_response = decode_response(response)

    assert (
        isinstance(decoded_response, dict)
        and len(decoded_response) == 1
        and expected_variant in decoded_response
    ), f"Expecting a dict with a single key '{expected_variant}', but got: {decoded_response!r}"

    return decoded_response[expected_variant]
