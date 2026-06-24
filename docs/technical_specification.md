# Mintlayer application: Technical Specifications

This page details the protocol implemented for the Mintlayer Ledger app.

## Framework

### APDUs

The messaging format of the app is compatible with the standard [APDU protocol](https://developers.ledger.com/docs/device-app/explanation/io#apdu-interpretation-loop).

The class byte used for all standard commands is `CLA = 0xE1`.

| CLA | INS | COMMAND NAME   | DESCRIPTION                                                        |
| --- | --- | -------------- | ------------------------------------------------------------------ |
| E1  | 00  | GET_PUBLIC_KEY | Return (and optionally show on screen) a public key and chain code |
| E1  | 01  | SIGN_TX        | Sign a transaction through a multi-step process                    |
| E1  | 02  | SIGN_MSG       | Sign a generic message                                             |
| E1  | 03  | PING           | Simple ping request to check connectivity                          |

### Chunking and P2

The APDU buffer on the Ledger device has a maximum data length (`MAX_APDU_DATA_LEN` = 255 bytes). To accommodate larger payloads, the app implements a chunking mechanism controlled by the `P2` parameter:

- `P2 = 0x00` (`P2_DONE`): This is the final chunk (or the only chunk) of the instruction. The app will assemble the buffer and execute the command.
- `P2 = 0x80` (`P2_MORE`): More chunks follow. The app accumulates the data into a buffer (up to a maximum of `4080` bytes) and returns SCALE-encoded `Response::ExpectingNextChunk` to ask the client for the next chunk.

_Note: For chunked commands, `INS` and `P1` must remain identical across all chunks of the same sequence._

### Data Serialization and Responses

The app uses **SCALE codec** (`parity_scale_codec`) for the serialization and deserialization of the data payloads. Vector length prefixes and standard variable-length integer types are represented using SCALE's Compact encoding.

#### Response Wrapper

On a successful command execution (returning status word `0x9000`), the app encodes the returned payload inside a top-level SCALE-encoded `Response` enum. 

The SCALE representation of an enum begins with a single byte representing the variant index, followed by the encoded fields of that variant.

The `Response` enum structure and its variant indices are:

| Index | Name                 | Inner Payload Type           | Description                                                            |
|-------|----------------------|------------------------------|------------------------------------------------------------------------|
| `0`   | `ExpectingNextChunk` | None                         | Returned when more APDU chunks are expected (`P2_MORE` sequence)       |
| `1`   | `PublicKey`          | `PublicKeyResponse`          | Public key and chain code response                                     |
| `2`   | `TxSetup`            | None                         | Acknowledges transaction initialization (`SIGN_TX` with `P1 = 0`)      |
| `3`   | `TxNext`             | None                         | Acknowledges receipt of a transaction chunk                            |
| `4`   | `TxInputSignature`   | `TxInputSignatureResponse`   | Contains an input signature                                            |
| `5`   | `MessageSetup`       | None                         | Acknowledges message signing initialization (`SIGN_MSG` with `P1 = 0`) |
| `6`   | `MessageSignature`   | `MsgSignatureResponse`       | Contains the final message signature                                   |
| `7`   | `Pong`               | None                         | Pong response for the `PING` instruction                               |

Any successful command output described below is prefixed by its corresponding 1-byte variant index.

## Status Words

The application returns standard Ledger status words as well as app-specific and ECC cryptographic errors.

| SW     | SW name                        | Description                                    |
| ------ | ------------------------------ | ---------------------------------------------- |
| 0x5515 | `DeviceLocked`                 | Device locked                                  |
| 0x6982 | `NothingReceived`              | Nothing received                               |
| 0x6985 | `Deny`                         | Rejected by user / User cancelled              |
| 0x6D00 | `Unknown`                      | Unknown                                        |
| 0x6E00 | `ClaNotSupported`              | CLA not supported                              |
| 0x6E01 | `InsNotSupported`              | Instruction not supported                      |
| 0x6E02 | `WrongP1P2`                    | Wrong P1/P2 parameters                         |
| 0x6E03 | `WrongApduLength`              | Wrong APDU length                              |
| 0x9000 | `Ok`                           | Success                                        |
| 0xB000 | `TxDisplayFail`                | Transaction display failed                     |
| 0xB001 | `TxLockTimeInvalid`            | Transaction lock time value is invalid         |
| 0xB002 | `TxWrongLength`                | Transaction wrong length                       |
| 0xB003 | `TxHashFail`                   | Transaction hashing failed                     |
| 0xB004 | `TxAddressFail`                | Transaction address failed                     |
| 0xB005 | `WrongInstruction`             | Different instruction than expected            |
| 0xB006 | `KeyDeriveFail`                | Key derivation failed                          |
| 0xB007 | `OrdersV0NotSupported`         | Orders V0 not supported                        |
| 0xB008 | `WrongContext`                 | Wrong context (e.g., Next called before Start) |
| 0xB009 | `DeserializeFail`              | SCALE deserialization failed                   |
| 0xB00A | `TxInvalidInputUtxo`           | Invalid input UTXO                             |
| 0xB00B | `TxNumericOperationFail`       | Numeric operation failed                       |
| 0xB00C | `TxFeeUnderflow`               | Tx fee underflow                               |
| 0xB00D | `InvalidData`                  | Invalid data                                   |
| 0xB00E | `NothingToSign`                | Nothing to sign                                |
| 0xB00F | `TxAlreadyFinished`            | Transaction already finished                   |
| 0xB010 | `InvalidPath`                  | Invalid BIP32 path                             |
| 0xB011 | `InvalidUncompressedPublicKey` | Invalid uncompressed public key                |
| 0xB012 | `MaxBufferLenExceeded`         | Max buffer length exceeded (Chunking limit)    |
| 0xB013 | `DifferentInputCommitmentHash` | Different input commitment hash                |
| 0xB014 | `InvalidTimestamp`             | Invalid timestamp                              |
| 0xB100 | `EccCarry`                     | ECC Carry                                      |
| 0xB101 | `EccLocked`                    | ECC Locked                                     |
| 0xB102 | `EccUnlocked`                  | ECC Unlocked                                   |
| 0xB103 | `EccNotLocked`                 | ECC Not Locked                                 |
| 0xB104 | `EccNotUnlocked`               | ECC Not Unlocked                               |
| 0xB105 | `EccInternalError`             | ECC Internal Error                             |
| 0xB106 | `EccInvalidParameterSize`      | ECC Invalid Parameter Size                     |
| 0xB107 | `EccInvalidParameterValue`     | ECC Invalid Parameter Value                    |
| 0xB108 | `EccInvalidParameter`          | ECC Invalid Parameter                          |
| 0xB109 | `EccNotInvertible`             | ECC Not Invertible                             |
| 0xB10A | `EccOverflow`                  | ECC Overflow                                   |
| 0xB10B | `EccMemoryFull`                | ECC Memory Full                                |
| 0xB10C | `EccNoResidue`                 | ECC No Residue                                 |
| 0xB10D | `EccPointAtInfinity`           | ECC Point At Infinity                          |
| 0xB10E | `EccInvalidPoint`              | ECC Invalid Point                              |
| 0xB10F | `EccInvalidCurve`              | ECC Invalid Curve                              |
| 0xB110 | `EccGenericError`              | ECC Generic Error                              |
| 0xE000 | `Panic`                        | Panic                                          |

---

## Commands

### GET_PUBLIC_KEY

Returns a public key and chain code at the given derivation path.
Optionally displays the generated address on the device screen for user verification.

#### Encoding

**Command**

| _CLA_ | _INS_ | _P1_                 |
| ----- | ----- | -------------------- |
| E1    | 00    | `0` or `1` (Display) |

**Input data (`GetPubKeyReq` - SCALE encoded)**

| Type        | Name        | Description                                               |
| ----------- | ----------- | --------------------------------------------------------- |
| `u8` (Enum) | `coin_type` | `0` = Mainnet, `1` = Testnet, `2` = Regtest, `3` = Signet |
| `Vec<u32>`  | `path`      | The BIP32 derivation path                                 |

**Output data (`Response::PublicKey(PublicKeyResponse)` - SCALE encoded)**

| Length | Description                                                         |
| ------ | ------------------------------------------------------------------- |
| `1`    | Variant index byte (`0x01`)                                         |
| `65`   | The uncompressed public key (`PublicKeyResponse.public_key`)        |
| `32`   | The chain code (`PublicKeyResponse.chain_code`)                     |

#### Description

If `P1` is `0`, the application derives the public key and returns it silently.
If `P1` is `1`, the application will display the address derived from the requested path on the device screen.
The command will only return `Ok` (0x9000) if the user approves it, otherwise, it returns `Deny` (0x6985).

---

### SIGN_TX

Signs a Mintlayer transaction.
Because transactions can be larger than available APDU buffers and RAM, the parsing and signing process is split into an interactive state machine using `P1 = 0` (Start) and `P1 = 1` (Next).

#### Encoding

**Command**

| _CLA_ | _INS_ | _P1_                      |
| ----- | ----- | ------------------------- |
| E1    | 01    | `0` (Start) or `1` (Next) |

**Input Data for `P1 = 0` (Start) (`SignTxStartReq` - SCALE encoded)**

| Type  | Name          | Description                                               |
| ----- | ------------- | --------------------------------------------------------- |
| `u8`  | `coin`        | `0` = Mainnet, `1` = Testnet, `2` = Regtest, `3` = Signet |
| `u8`  | `version`     | Transaction version (0 = V1)                            |
| `u32` | `num_inputs`  | Total number of inputs in the transaction                 |
| `u32` | `num_outputs` | Total number of outputs in the transaction                |

**Input Data for `P1 = 1` (Next) (`SignTxNextReq` Enum - SCALE encoded)**

The client sends a sequence of `SignTxNextReq` variants. The variant index dictates the type of data being sent:

- `ProcessInput` (Index 0): Contains `TxInputData` (input address paths and UTXO/Account info).
- `ProcessInputCommitment` (Index 1): Contains `TxInputCommitmentData`.
- `ProcessOutput` (Index 2): Contains `TxOutputData`.
- `ReturnNextSignature` (Index 3): Requests the device to yield the next available signature.

**Output data (`Response::TxInputSignature(TxInputSignatureResponse)` - SCALE encoded)**

Yielded during `ReturnNextSignature` sequences.

The response payload is prefixed with the `TxInputSignature` variant index (`0x04`), followed by the `TxInputSignatureResponse` fields:

| Type          | Name           | Description                                 |
| ------------- | -------------- | ------------------------------------------- |
| `[u8; 64]`    | `signature`    | The 64-byte cryptographic signature         |
| `u32`         | `input_idx`    | The index of the input that was just signed |
| `Option<u32>` | `multisig_idx` | Optional multisig index                     |
| `bool`        | `has_next`     | True if there are more signatures remaining |

*Note: For `Start` (`P1 = 0`) and intermediate `Next` (`P1 = 1`) data chunks (before signatures), the app returns `Response::TxSetup` (variant index `0x02`) and `Response::TxNext` (variant index `0x03`) respectively, with no extra fields.*

#### Description

To sign a transaction, the client must follow a strict order:

1. Call `SIGN_TX` with `P1 = 0` (Start) passing `SignTxStartReq`.
2. Sequentially call `SIGN_TX` with `P1 = 1` (Next) to stream inputs (`ProcessInput`), input commitments (`ProcessInputCommitment`) and then outputs (`ProcessOutput`).
3. After all data is verified by the user on the device's secure screen, the client requests signatures by repeatedly calling `SIGN_TX` with `P1 = 1` and the `ReturnNextSignature` variant.
4. The device will yield `TxInputSignatureResponse` payloads until `has_next` is false.

---

### SIGN_MSG

Signs a generic message using a BIP-32 derived key.

#### Encoding

**Command**

| _CLA_ | _INS_ | _P1_                      |
| ----- | ----- | ------------------------- |
| E1    | 02    | `0` (Start) or `1` (Next) |

**Input Data for `P1 = 0` (Start) (`SignMessageStartReq` - SCALE encoded)**

| Type        | Name        | Description                                   |
| ----------- | ----------- | --------------------------------------------- |
| `u8` (Enum) | `coin`      | Coin type (Mainnet, Testnet, Regtest, Signet) |
| `u8` (Enum) | `addr_type` | `0` = PublicKey, `1` = PublicKeyHash          |
| `Vec<u32>`  | `path`      | The BIP32 derivation path to use for signing  |

**Input Data for `P1 = 1` (Next)**

| Type         | Description                                                   |
| ------------ | ------------------------------------------------------------- |
| `<variable>` | The raw byte chunks of the message payload to be accumulated. |

**Output data (`Response::MessageSignature(MsgSignatureResponse)` - SCALE encoded)**

The response payload is prefixed with the `MessageSignature` variant index (`0x06`), followed by the `MsgSignatureResponse` fields:

| Length | Description                         |
| ------ | ----------------------------------- |
| `64`   | The 64-byte cryptographic signature |

*Note: For `Start` (`P1 = 0`) initialization, the app returns `Response::MessageSetup` (variant index `0x05`) with no extra fields.*

#### Description

The `SIGN_MSG` flow initializes the signing context via `Start`.
The client then sends the message chunks iteratively using `Next`.
Once the message transmission is completed and the user validates the request on the hardware screen,
the device returns the 64-byte signature.
Chunking the payload via the `P2` parameter mechanism is utilized here if the data exceeds a single APDU packet limit.

---

### PING

Simple ping request to check connectivity.

#### Encoding

**Command**

| _CLA_ | _INS_ | _P1_ |
| ----- | ----- | ---- |
| E1    | 03    | `0`  |

**Input data**

None.

**Output data (`Response::Pong` - SCALE encoded)**

| Length | Description                                                 |
| ------ | ----------------------------------------------------------- |
| `1`    | Variant index byte (`0x07`) representing `Response::Pong` |

#### Description

The client sends a `PING` command to verify that the app is running and responsive.
The app returns `Response::Pong` (success indicator) on success, and returns the home screen to the user.
