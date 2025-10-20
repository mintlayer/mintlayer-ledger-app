from pathlib import Path

import tomli

from application_client.mintlayer_command_sender import MintlayerCommandSender
from application_client.mintlayer_response_unpacker import \
    unpack_get_version_response


# In this test we check the behavior of the device when asked to provide the app version
def test_version(backend):

    with open(Path(__file__).parent.parent / "Cargo.toml", "rb") as f:
        data = tomli.load(f)
    version = tuple(map(int, data["package"]["version"].split(".")))
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    # Send the GET_VERSION instruction
    rapdu = client.get_version()
    # Use an helper to parse the response, assert the values
    assert unpack_get_version_response(rapdu.data) == (version)
