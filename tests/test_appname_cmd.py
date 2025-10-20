from application_client.mintlayer_command_sender import MintlayerCommandSender
from application_client.mintlayer_response_unpacker import \
    unpack_get_app_name_response


# In this test we check that the GET_APP_NAME replies the application name
def test_app_name(backend):
    # Use the app interface instead of raw interface
    client = MintlayerCommandSender(backend)
    # Send the GET_APP_NAME instruction to the app
    response = client.get_app_name()
    # Assert that we have received the correct appname
    assert unpack_get_app_name_response(response.data) == "mintlayer-app"
