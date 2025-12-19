from dataclasses import dataclass
from typing import List


class TransactionError(Exception):
    pass


@dataclass
class Transaction:
    coin: int
    inputs: List[bytes]
    input_commitments: List[bytes]
    outputs: List[bytes]
