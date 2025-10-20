from dataclasses import dataclass
from hashlib import blake2b
from typing import List

from scalecodec.types import CompactU32  # type: ignore


class TransactionError(Exception):
    pass


@dataclass
class Transaction:
    coin: int
    inputs: List[bytes]
    input_additional_data: List[bytes]
    outputs: List[bytes]

    def to_hash(self) -> bytes:
        hasher = blake2b()
        hasher.update(b"\x01\x01")
        hasher.update(b"\x00" * 16)
        hasher.update(len(self.inputs).to_bytes(4, "little"))
        for inp in self.inputs:
            hasher.update(inp)

        hasher.update(len(self.inputs).to_bytes(4, "little"))
        for inp_com in self.input_additional_data:
            hasher.update(inp_com)

        hasher.update(CompactU32().encode(len(self.outputs)).data)
        for out in self.outputs:
            hasher.update(out)

        h = hasher.digest()[:32]
        return blake2b(h).digest()[:32]
