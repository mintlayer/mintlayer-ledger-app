from struct import unpack
from typing import Tuple

import scalecodec  # type: ignore


# remainder, data_len, data
def pop_sized_buf_from_buffer(buffer: bytes, size: int) -> Tuple[bytes, bytes]:
    return buffer[size:], buffer[0:size]


# remainder, data_len, data
def pop_size_prefixed_buf_from_buf(buffer: bytes) -> Tuple[bytes, int, bytes]:
    data_len = buffer[0]
    return buffer[1 + data_len :], data_len, buffer[1 : data_len + 1]


# Unpack from response:
# response = app_name (var)
def unpack_get_app_name_response(response: bytes) -> str:
    return response.decode("ascii")


# Unpack from response:
# response = MAJOR (1)
#            MINOR (1)
#            PATCH (1)
def unpack_get_version_response(response: bytes) -> Tuple[int, int, int]:
    assert len(response) == 3
    major, minor, patch = unpack("BBB", response)
    return (major, minor, patch)


# Unpack from response:
# response = format_id (1)
#            app_name_raw_len (1)
#            app_name_raw (var)
#            version_raw_len (1)
#            version_raw (var)
#            unused_len (1)
#            unused (var)
def unpack_get_app_and_version_response(response: bytes) -> Tuple[str, str]:
    response, _ = pop_sized_buf_from_buffer(response, 1)
    response, _, app_name_raw = pop_size_prefixed_buf_from_buf(response)
    response, _, version_raw = pop_size_prefixed_buf_from_buf(response)
    response, _, _ = pop_size_prefixed_buf_from_buf(response)

    assert len(response) == 0

    return app_name_raw.decode("ascii"), version_raw.decode("ascii")


# Unpack from response:
def unpack_get_public_key_response(response: bytes) -> Tuple[int, bytes, int, bytes]:
    response_bytes = scalecodec.base.ScaleBytes(response)
    response_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
        "Response", data=response_bytes
    )
    msg = response_obj.decode()

    print(msg)

    pub_key = bytes.fromhex(msg["PublicKey"]["public_key"][2:])
    pub_key_len = len(pub_key)
    chain_code = bytes.fromhex(msg["PublicKey"]["chain_code"][2:])
    chain_code_len = len(chain_code)

    print(pub_key_len, pub_key)
    print(chain_code_len, chain_code)

    assert pub_key_len == 65
    assert chain_code_len == 32
    return pub_key_len, pub_key, chain_code_len, chain_code


# Unpack from response:
def unpack_sign_message_response(response: bytes) -> bytes:
    response_bytes = scalecodec.base.ScaleBytes(response)
    response_obj = scalecodec.base.RuntimeConfiguration().create_scale_object(
        "Response", data=response_bytes
    )
    resp = response_obj.decode()
    assert resp["MessageSignature"] is not None
    signature = bytes.fromhex(resp["MessageSignature"]["signature"][2:])
    assert len(signature) == 64
    return signature
