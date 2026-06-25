# How to use the Ragger test framework

This document describes how to run the Mintlayer Ledger app functional tests with the Ragger test
framework, either on the Speculos emulator or on a physical Ledger device.

## Quickly get started with Ragger and Speculos

### Install Ragger and dependencies

In this document we'll be running tests outside of a Docker container, in which case additional
dependencies must be installed:

```bash
pip install -r tests/requirements.txt
sudo apt-get update && sudo apt-get install qemu-user-static
```

### Build the application

Build the app for the device model that will be used by the tests; you can use the image
`ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder` for this:

```bash
docker pull ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder:latest
docker run --user "$(id -u)":"$(id -g)" --rm -ti -v "$(realpath .):/app" -w /app ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder:latest
```

Then, inside the container:

```bash
cargo ledger build nanox
```

ℹ️ `cargo ledger build` accepts `nanox`, `nanosplus`, `stax`, `flex`, and `apex_p`.

ℹ️ Alternatively, the `ledger-app-dev-tools` image can be used. It already contains tools required to run tests
and must be used if you intend to run them inside the Docker container.

### Run tests using the Speculos emulator

Run the functional tests from the repository root:

```bash
pytest tests/ --tb=short -v --device nanox
```

To see the emulator display while tests run, add `--display`:

```bash
pytest tests/ --tb=short -v --device nanox --display
```

⚠️ `--device` specifies the Speculos device model and it should match the binary you've built.
Normally, it will be the same as the `cargo` target that you've passed to `cargo ledger build`,
except for `nanosplus`, whose corresponding device model name is `nanosp`.

### Run tests using a physical Ledger device

The Mintlayer app must be built, loaded on the device, and opened before running the tests.
See [Loading on device](../README.md#loading-on-device) for platform-specific loading instructions.

After the app has been installed on the device, run the tests via:
```bash
pytest tests/ --tb=short -v --device nanox --backend ledgerwallet
```

⚠️ Functional tests expect the device to be seeded with the seed phrase "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".
If this is not the case, most of the tests will fail.

## Available pytest options

Standard useful pytest options:

```text
    -v              formats the test summary in a readable way;
    -s              enables logs for successful tests; on Speculos it enables app logs if compiled with DEBUG=1;
    -k <expression> only runs the tests which match <expression>; the expression may be a single substring to match
                    against test names and their parent classes, or a combination of them, e.g. '-k foo' or
                    '-k "(foo or bar) and not baz"';
    --tb=short      formats tracebacks in a compact way;
```

Custom pytest options:

```text
    --device <device>           run tests on the specified device [nanox,nanosp,stax,flex,apex_p,all];
    --backend <backend>         run tests against [speculos, ledgercomm, ledgerwallet]; Speculos is the default;
    --display                   on Speculos, show the app screen using QT;
    --golden_run                save current screens instead of comparing them;
    --log_apdu_file <filepath>  log all APDU exchanges to the given file; previous file content is erased;
```
