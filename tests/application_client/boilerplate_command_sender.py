from enum import IntEnum
from typing import Generator, List, Optional
from contextlib import contextmanager

import scalecodec

from ragger.backend.interface import BackendInterface, RAPDU

from .boilerplate_transaction import Transaction


MAX_APDU_LEN: int = 255

CLA: int = 0xE0

class P1(IntEnum):
    # Parameter 1 for first APDU number.
    P1_START = 0x00
    P1_TX_INPUT = 0x01
    P1_TX_INPUT_COMMITMENT = 0x02
    P1_TX_OUTPUT = 0x03
    P1_TX_NEXT_SIG = 0x04
    # Parameter 1 for maximum APDU number.
    P1_MAX   = 0x03
    # Parameter 1 for screen confirmation for GET_PUBLIC_KEY.
    P1_CONFIRM = 0x01

class P2(IntEnum):
    # Parameter 2 for last APDU to receive.
    P2_LAST = 0x00
    # Parameter 2 for more APDU to receive.
    P2_MORE = 0x80

class InsType(IntEnum):
    GET_VERSION    = 0x03
    GET_APP_NAME   = 0x04
    GET_PUBLIC_KEY = 0x05
    SIGN_TX        = 0x06
    SIGN_MESSAGE   = 0x07

class Errors(IntEnum):
    SW_DENY                    = 0x6985
    SW_WRONG_P1P2              = 0x6A86
    SW_INS_NOT_SUPPORTED       = 0x6D00
    SW_CLA_NOT_SUPPORTED       = 0x6E00
    SW_WRONG_APDU_LENGTH       = 0x6E03
    SW_WRONG_RESPONSE_LENGTH   = 0xB000
    SW_DISPLAY_BIP32_PATH_FAIL = 0xB001
    SW_DISPLAY_ADDRESS_FAIL    = 0xB002
    SW_DISPLAY_AMOUNT_FAIL     = 0xB003
    SW_WRONG_TX_LENGTH         = 0xB004
    SW_TX_PARSING_FAIL         = 0xB005
    SW_TX_HASH_FAIL            = 0xB006
    SW_BAD_STATE               = 0xB007
    SW_SIGNATURE_FAIL          = 0xB008


def split_message(message: bytes, max_size: int) -> List[bytes]:
    return [message[x:x + max_size] for x in range(0, len(message), max_size)]


class BoilerplateCommandSender:
    def __init__(self, backend: BackendInterface) -> None:
        self.backend = backend


    def get_app_and_version(self) -> RAPDU:
        return self.backend.exchange(cla=0xB0,  # specific CLA for BOLOS
                                     ins=0x01,  # specific INS for get_app_and_version
                                     p1=P1.P1_START,
                                     p2=P2.P2_LAST,
                                     data=b"")


    def get_version(self) -> RAPDU:
        return self.backend.exchange(cla=CLA,
                                     ins=InsType.GET_VERSION,
                                     p1=P1.P1_START,
                                     p2=P2.P2_LAST,
                                     data=b"")


    def get_app_name(self) -> RAPDU:
        return self.backend.exchange(cla=CLA,
                                     ins=InsType.GET_APP_NAME,
                                     p1=P1.P1_START,
                                     p2=P2.P2_LAST,
                                     data=b"")


    def get_public_key(self, coin: int, path: str) -> RAPDU:
        data = coin.to_bytes(1) + pack_derivation_path(path)

        return self.backend.exchange(cla=CLA,
                                     ins=InsType.GET_PUBLIC_KEY,
                                     p1=P1.P1_START,
                                     p2=P2.P2_LAST,
                                     data=data)


    @contextmanager
    def get_public_key_with_confirmation(self, coin: int, path: str) -> Generator[None, None, None]:
        data = coin.to_bytes(1) + pack_derivation_path(path)

        with self.backend.exchange_async(cla=CLA,
                                         ins=InsType.GET_PUBLIC_KEY,
                                         p1=P1.P1_CONFIRM,
                                         p2=P2.P2_LAST,
                                         data=data) as response:
            yield response

    @contextmanager
    def sign_message(self, coin: int, path: str, message: bytes) -> Generator[None, None, None]:
        data = coin.to_bytes(1) + pack_derivation_path(path)
        
        self.backend.exchange(cla=CLA,
                              ins=InsType.SIGN_MESSAGE,
                              p1=P1.P1_START,
                              p2=P2.P2_MORE,
                              data=data)
        messages = split_message(message, MAX_APDU_LEN)
        idx: int = P1.P1_START + 1

        for msg in messages[:-1]:
            self.backend.exchange(cla=CLA,
                                  ins=InsType.SIGN_MESSAGE,
                                  p1=idx,
                                  p2=P2.P2_MORE,
                                  data=msg)
            idx += 1

        with self.backend.exchange_async(cla=CLA,
                                         ins=InsType.SIGN_MESSAGE,
                                         p1=idx,
                                         p2=P2.P2_LAST,
                                         data=messages[-1]) as response:
            yield response

    @contextmanager
    def sign_tx(self, transaction: Transaction) -> Generator[None, None, None]:
        metadata = bytes([
            #1 + 1 + 4 + 4, # len
            transaction.coin,
            1, # version
            ]) + len(transaction.inputs).to_bytes(byteorder="big", length=4) + len(transaction.outputs).to_bytes(byteorder="big", length=4)
        print("metadata ", len(metadata))

        res = self.backend.exchange(cla=CLA,
                              ins=InsType.SIGN_TX,
                              p1=P1.P1_START,
                              p2=P2.P2_MORE,
                              data=bytes(metadata))
        print("metadata ", res)

        for inp in transaction.inputs:
            res = self.backend.exchange(cla=CLA,
                                    ins=InsType.SIGN_TX,
                                    p1=P1.P1_TX_INPUT,
                                    p2=P2.P2_LAST,
                                    data=inp[0])
            print("inp M ", res)

            res = self.backend.exchange(cla=CLA,
                                    ins=InsType.SIGN_TX,
                                    p1=P1.P1_TX_INPUT,
                                    p2=P2.P2_LAST,
                                    data=inp[1])
            print("inp ", res)

        for inpc in transaction.input_commitements:
            res = self.backend.exchange(cla=CLA,
                                    ins=InsType.SIGN_TX,
                                    p1=P1.P1_TX_INPUT_COMMITMENT,
                                    p2=P2.P2_LAST,
                                    data=inpc)
            print("inpC ", res)

        for out in transaction.outputs[:-1]:
            res = self.backend.exchange(cla=CLA,
                                    ins=InsType.SIGN_TX,
                                    p1=P1.P1_TX_OUTPUT,
                                    p2=P2.P2_LAST,
                                    data=out)
            print("Out ", res)

        with self.backend.exchange_async(cla=CLA,
                                         ins=InsType.SIGN_TX,
                                         p1=P1.P1_TX_OUTPUT,
                                         p2=P2.P2_LAST,
                                         data=transaction.outputs[-1]) as response:
            yield response

            

        """
        self.backend.exchange(cla=CLA,
                              ins=InsType.SIGN_TX,
                              p1=P1.P1_START,
                              p2=P2.P2_MORE,
                              data=pack_derivation_path(path))
        messages = split_message(transaction, MAX_APDU_LEN)
        idx: int = P1.P1_START + 1

        for msg in messages[:-1]:
            self.backend.exchange(cla=CLA,
                                  ins=InsType.SIGN_TX,
                                  p1=idx,
                                  p2=P2.P2_MORE,
                                  data=msg)
            idx += 1

        
        """

    def get_async_response(self) -> Optional[RAPDU]:
        return self.backend.last_async_response
    
    def get_all_signatures(self, tx: Transaction) -> List[RAPDU]:
        responses = [self.backend.last_async_response.data]
        for _ in tx.inputs[1:]:
            res = self.backend.exchange(cla=CLA,
                                    ins=InsType.SIGN_TX,
                                    p1=4,
                                    p2=2,
                                    data=bytes())
            responses.append(res.data)
        return responses

def hardened_index(index: int) -> int:
    return index | 1 << 31

def pack_derivation_path(derivation_path: str) -> bytes:
    path_obj = scalecodec.base.RuntimeConfiguration().create_scale_object('Bip32Path')

    split = derivation_path.split("/")


    if split[0] != "m":
        raise ValueError("Error master expected")

    path = []
    for value in split[1:]:
        if value == "":
            raise ValueError(f'Error missing value in split list "{split}"')
        if value.endswith('\''):
            path.append(hardened_index(int(value[:-1])))
        else:
            path.append(int(value))
        
    return path_obj.encode(path).data