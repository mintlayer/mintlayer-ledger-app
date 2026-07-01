from application_client.mintlayer_command_sender import MintlayerCommandSender
from application_client.mintlayer_response_unpacker import (
    unpack_get_app_and_version_response,
)


# Test a specific APDU asking BOLOS (and not the app) the name and version of the current app
def test_get_app_and_version(backend, backend_name):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    # Send the special instruction to BOLOS
    response = client.get_app_and_version()
    # Use an helper to parse the response, assert the values
    app_name, version = unpack_get_app_and_version_response(response.data)
    print(app_name, version)
    assert app_name == "Mintlayer"
    assert version == "1.0.0"
