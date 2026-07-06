from dataclasses import dataclass
from enum import IntEnum
from hashlib import blake2b
from typing import List, Optional

from coincurve import PublicKeyXOnly
import scalecodec  # type: ignore

sign_tx_start_req_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
    "SignTxStartReq"
)
sign_tx_next_req_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
    "SignTxNextReq"
)
compact_u32_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
    "Compact<u32>"
)
tx_input_obj = scalecodec.base.RuntimeConfiguration().create_scale_object("TxInput")
tx_input_commitment_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
    "SighashInputCommitment"
)
tx_output_obj = scalecodec.base.RuntimeConfiguration().create_scale_object("TxOutput")


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
    coin_type: int
    # A list of TxInputData objects.
    inputs: List[dict]
    # A list of TxInputCommitmentData objects.
    input_commitments: List[dict]
    # A list of TxOutputData objects.
    outputs: List[dict]

    # Extract key derivation path for each input_idx/multisig_idx combination
    # and return it as a dict.
    def addr_paths_by_indices(self) -> dict[TxInputSignatureIndices, list[int]]:
        result = {}
        for input_idx, inp in enumerate(self.inputs):
            for addr in inp["addresses"]:
                multisig_idx = addr["multisig_idx"]
                indices = TxInputSignatureIndices(
                    input_idx=input_idx, multisig_idx=multisig_idx
                )

                result[indices] = addr["path"]

        return result

    def expected_sig_indices(self) -> set[TxInputSignatureIndices]:
        return set(self.addr_paths_by_indices())

    def digest_for_signing(self) -> bytes:
        encoded_inputs = b""
        for inp_data in self.inputs:
            inp_with_info = inp_data["input"]
            if "Utxo" in inp_with_info:
                inp = {"Utxo": inp_with_info["Utxo"][0]}
            elif "Account" in inp_with_info:
                inp = {"Account": inp_with_info["Account"]}
            elif "AccountCommand" in inp_with_info:
                inp = {"AccountCommand": inp_with_info["AccountCommand"]}
            elif "OrderAccountCommand" in inp_with_info:
                inp = {"OrderAccountCommand": inp_with_info["OrderAccountCommand"][0]}
            else:
                raise ValueError("Unexpected input")

            encoded_inputs += tx_input_obj.encode(inp).data

        encoded_input_commitments = b""
        for inp_comm in self.input_commitments:
            comm = inp_comm["commitment"]
            encoded_input_commitments += tx_input_commitment_obj.encode(comm).data

        encoded_outputs = b""
        for outp_data in self.outputs:
            outp = outp_data["output"]
            encoded_outputs += tx_output_obj.encode(outp).data

        assert len(self.input_commitments) == len(self.inputs)

        num_inputs_as_le_bytes = len(self.inputs).to_bytes(4, "little")
        compact_encoded_num_outputs = compact_u32_obj.encode(len(self.outputs)).data

        preimage = (
            b"\x01"  # SigHashType::ALL
            + b"\x01"  # version
            + bytes(16)  # zero flags
            + num_inputs_as_le_bytes
            + encoded_inputs
            + num_inputs_as_le_bytes
            + encoded_input_commitments
            + compact_encoded_num_outputs
            + encoded_outputs
        )

        tx_hash = mintlayer_hash(mintlayer_hash(preimage))
        return tx_hash


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


def mintlayer_hash(data: bytes) -> bytes:
    return blake2b(data, digest_size=64).digest()[:32]


def hardened_index(index: int) -> int:
    return index | 1 << 31


class KeyPurpose(IntEnum):
    Receive = 0
    Change = 1


def make_path(account_index: int, key_purpose: KeyPurpose, key_index: int) -> list[int]:
    return [
        hardened_index(44),
        hardened_index(19788),
        hardened_index(account_index),
        key_purpose,
        key_index,
    ]


def parse_derivation_path(derivation_path: str) -> list[int]:
    split = derivation_path.split("/")

    if split[0] != "m":
        raise ValueError("Error master expected")

    result = []
    for value in split[1:]:
        if value == "":
            raise ValueError(f'Error missing value in split list "{split}"')
        if value.endswith("'"):
            result.append(hardened_index(int(value[:-1])))
        else:
            result.append(int(value))

    return result


MESSAGE_MAGIC_PREFIX = b"===MINTLAYER MESSAGE BEGIN===\n"
MESSAGE_MAGIC_SUFFIX = b"\n===MINTLAYER MESSAGE END==="


def mintlayer_message_digest(message: bytes) -> bytes:
    framed = MESSAGE_MAGIC_PREFIX + message + MESSAGE_MAGIC_SUFFIX
    return mintlayer_hash(mintlayer_hash(framed))


def xonly_pubkey(public_key: bytes) -> bytes:
    if len(public_key) == 32:
        return public_key
    if len(public_key) == 33 and public_key[0] in (0x02, 0x03):
        return public_key[1:33]
    if len(public_key) == 65 and public_key[0] == 0x04:
        return public_key[1:33]
    raise ValueError(
        "Expected x-only, compressed, or uncompressed secp256k1 public key"
    )


def verify_message_signature(msg: bytes, pubkey: bytes, sig: bytes) -> bool:
    digest = mintlayer_message_digest(msg)
    return PublicKeyXOnly(xonly_pubkey(pubkey)).verify(sig, digest)


def verify_tx_signature(tx: Transaction, pubkey: bytes, sig: bytes) -> bool:
    digest = tx.digest_for_signing()
    return PublicKeyXOnly(xonly_pubkey(pubkey)).verify(sig, digest)
