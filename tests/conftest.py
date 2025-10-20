import pytest
from ragger.conftest import configuration
from ragger.navigator import NavInsID

###########################
### CONFIGURATION START ###
###########################

# You can configure optional parameters by overriding the value of ragger.configuration.OPTIONAL_CONFIGURATION
# Please refer to ragger/conftest/configuration.py for their descriptions and accepted values

configuration.OPTIONAL.CUSTOM_SEED = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

#########################
### CONFIGURATION END ###
#########################

# Pull all features from the base ragger conftest using the overridden configuration
pytest_plugins = ("ragger.conftest.base_conftest",)
