import json
from dataclasses import dataclass
from .boilerplate_utils import UINT64_MAX
from typing import List, Tuple
from hashlib import blake2b
from scalecodec.types import CompactU32

class TransactionError(Exception):
    pass

@dataclass
class Transaction:
    inputs: List[Tuple[bytes, bytes]]
    input_commitements: List[bytes]
    outputs: List[bytes]


    def to_hash(self) -> bytes:
        hasher = blake2b()
        hasher.update(b"\x01\x01")
        hasher.update(b"\x00"*16)
        hasher.update(len(self.inputs).to_bytes(4, "little"))
        for inp in self.inputs:
            hasher.update(inp[1])

        hasher.update(len(self.inputs).to_bytes(4, "little"))
        for inp in self.input_commitements:
            hasher.update(inp)

        
        hasher.update(CompactU32().encode(len(self.outputs)).data)
        for out in self.outputs:
            hasher.update(out)

        h = hasher.digest()[:32]
        return blake2b(h).digest()[:32]