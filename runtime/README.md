# Caliptra Runtime Firmware

## Runtime Firmware Environment

### Boot & Initialization

The Runtime Firmware main function SHALL perform the following on Cold Boot reset:

* Initialize DPE (see below for details)
* Initialize any SRAM structures used by Runtime Firmware

For behavior during other types of reset, see "Runtime Firmware Updates".

If `mailbox_flow_done` is not set during a "warm" "update" boot, it is assumed that Caliptra
was reset while runtime firmware was executing an operation. If Runtime firmware detects
this case, it will call `DISABLE_ATTESTATION`, since the internal state of Caliptra may
be corrupted.

### Main Loop

After booting, Caliptra Runtime firmware is responsible for the following:

* Wait for mailbox interrupts
* On mailbox interrupt
    * Unset `mailbox_flow_done`
    * Write lock mailbox and set busy register
    * Read command from mailbox
    * Execute command
    * Write response to mailbox and set necessary status register(s)
    * Set `mailbox_flow_done`
    * Sleep until next interrupt
* On panic
    * Save diagnostic information

Callers must wait until Caliptra is no longer busy to call a mailbox command.
Upon completion, Runtime Firmware will signal `mailbox_data_avail` to notify the
caller. Once the mailbox data has been read and the lock is released,
`mailbox_flow_done` will be signaled to notify callers that the mailbox is ready
for use.

### Fault Handling

A mailbox command can fail to complete in a couple ways

* Hang/timeout which results in the watchdog firing
* Unrecoverable panic

In both these cases, the panic handler will write diagnostic panic information
to registers readable by the SoC, firmware will undergo impactless reset, and
`mailbox_data_avail` will be asserted.

The caller is expected to check status registers upon reading responses from the
mailbox.

Depending on the type of fault, the SoC may:

* Resubmit the mailbox command
* Attempt to update Runtime Firmware
* Perform a full SoC reset
* Some other SoC-specific behavior

### Drivers

Caliptra Runtime Firmware will share driver code with ROM and FMC where
possible, however it will have its own copies of all these drivers linked into
the Runtime Firmware binary.

## Maibox Commands

All mailbox command codes are little endian.

Table: Mailbox command "result" codes:

| **Name**         | **Value**              | Description
| -------          | -----                  | -----------
| `SUCCESS`        | `0x0000_0000`          | Mailbox command succeeded
| `BAD_VENDOR_SIG` | `0x5653_4947` ("VSIG") | Vendor signature check failed
| `BAD_OWNER_SIG`  | `0x4F53_4947` ("OSIG") | Owner signature check failed
| `BAD_SIG`        | `0x4253_4947` ("BSIG") | Generic signature check failure (for crypto offload)
| `BAD_IMAGE`      | `0x4249_4D47` ("BIMG") | Malformed input image
| `BAD_CHKSUM`     | `0x4243_484B` ("BCHK") | Checksum check failed on input arguments

Relevant Registers:

* mbox\_csr -> COMMAND: Command code to execute
* mbox\_csr -> DLEN: Number of bytes written to mailbox
* CPTRA\_FW\_ERROR\_NON\_FATAL: Status code of mailbox command. Any result
  other than `SUCCESS` signifies a mailbox command failure.

### CALIPTRA\_FW\_LOAD

The `CALIPTRA_FW_LOAD` command is handled by both ROM and Runtime Firmware.

#### ROM Behavior

On cold boot, ROM will expose the `CALIPTRA_FW_LOAD` mailbox command to accept
the firmware image that ROM will boot. This includes Manifest, FMC, and Runtime
firmware.

#### Runtime Firmware Behavior

Caliptra Runtime FW will also expose this mailbox command for loading
impactless updates. See “Runtime Firmware Updates” for details.

Command Code: `0x4657_4C44` ("FWLD")

Table: `CALIPTRA_FW_LOAD` input arguments

| **Name**  | **Type**      | **Description**
| --------  | --------      | ---------------
| chksum    | u32           | Checksum over other input arguments, computed by the caller. Little endian.
| data      | u8[...]       | Firmware image to load.

Table: `CALIPTRA_FW_LOAD` output arguments

| **Name**    | **Type** | **Description**
| --------    | -------- | ---------------
| chksum      | u32      | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32      | Indicates if the command is FIPS approved or an error

### GET\_IDEV\_CERT

Exposes a command to reconstruct the IDEVID CERT

Command Code: `0x4944_4543` ("IDEC")

Table: `GET_IDEV_CERT` input arguments

| **Name**    | **Type**      | **Description**
| --------    | --------      | ---------------
| chksum      | u32           | Checksum over other input arguments, computed by the caller. Little endian.
| signature_r | u8[48]        | R portion of signature of the cert
| signature_s | u8[48]        | S portion of signature of the cert
| tbs_size    | u32           | Size of the TBS
| tbs         | u8[916]       | TBS, with a maximum size of 916. Only bytes up to tbs_size are used.

Table: `GET_IDEV_CERT` output arguments

| **Name**    | **Type**   | **Description**
| --------    | --------   | ---------------
| chksum      | u32        | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32        | Indicates if the command is FIPS approved or an error
| cert_size   | u32        | Length in bytes of the cert field in use for the IDevId certificate
| cert        | u8[1024]   | DER-encoded IDevID CERT

### POPULATE\_IDEV\_CERT

Exposes a command that allows the SoC to provide a DER-encoded
IDevId certificate on every boot. The IDevId certificate is added 
to the start of the certificate chain.

Command Code: `0x4944_4550` ("IDEP")

Table: `POPULATE_IDEV_CERT` input arguments

| **Name**    | **Type**      | **Description**
| --------    | --------      | ---------------
| chksum      | u32           | Checksum over other input arguments, computed by the caller. Little endian.
| cert_size   | u32           | Size of the DER-encoded IDevId certificate
| cert        | u8[1024]      | DER-encoded IDevID CERT

Table: `POPULATE_IDEV_CERT` output arguments

| **Name**    | **Type** | **Description**
| --------    | -------- | ---------------
| chksum      | u32      | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32      | Indicates if the command is FIPS approved or an error

### GET\_IDEV\_INFO

Exposes a command to get a IDEVID public key.

Command Code: `0x4944_4549` ("IDEI")

Table: `GET_IDEV_INFO` input arguments

| **Name**  | **Type**      | **Description**
| --------  | --------      | ---------------
| chksum    | u32           | Checksum over other input arguments, computed by the caller. Little endian.

Table: `GET_IDEV_INFO` output arguments

| **Name**    | **Type**   | **Description**
| --------    | --------   | ---------------
| chksum      | u32        | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32        | Indicates if the command is FIPS approved or an error
| idev_pub_x  | u8[48]     | X portion of ECDSA IDevId key
| idev_pub_y  | u8[48]     | Y portion of ECDSA IDevId key

### GET\_LDEV\_CERT

Exposes a command to get a self-signed LDevID Certificate signed by IDevID.

Command Code: `0x4C44_4556` ("LDEV")

Table: `GET_LDEV_CERT` input arguments

| **Name**  | **Type**      | **Description**
| --------  | --------      | ---------------
| chksum    | u32           | Checksum over other input arguments, computed by the caller. Little endian.

Table: `GET_LDEV_CERT` output arguments

| **Name**    | **Type**   | **Description**
| --------    | --------   | ---------------
| chksum      | u32        | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32        | Indicates if the command is FIPS approved or an error
| data_size   | u32        | Length in bytes of the valid data in the data field
| data        | u8[...]    | DER-encoded LDevID Certificate

### GET\_FMC\_ALIAS\_CERT

Exposes a command to get a self-signed FMC alias Certificate signed by LDevID.

Command Code: `0x4345_5246` ("CERF")

Table: `GET_FMC_ALIAS_CERT` input arguments

| **Name**  | **Type**      | **Description**
| --------  | --------      | ---------------
| chksum    | u32           | Checksum over other input arguments, computed by the caller. Little endian.

Table: `GET_FMC_ALIAS_CERT` output arguments

| **Name**    | **Type**   | **Description**
| --------    | --------   | ---------------
| chksum      | u32        | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32        | Indicates if the command is FIPS approved or an error
| data_size   | u32        | Length in bytes of the valid data in the data field
| data        | u8[...]    | DER-encoded FMC alias Certificate

### GET\_RT\_ALIAS\_CERT

Exposes a command to get a self-signed Runtime alias Certificate signed by the FMC alias.

Command Code: `0x4345_5252` ("CERR")

Table: `GET_RT_ALIAS_CERT` input arguments

| **Name**  | **Type**      | **Description**
| --------  | --------      | ---------------
| chksum    | u32           | Checksum over other input arguments, computed by the caller. Little endian.

Table: `GET_RT_ALIAS_CERT` output arguments

| **Name**    | **Type**   | **Description**
| --------    | --------   | ---------------
| chksum      | u32        | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32        | Indicates if the command is FIPS approved or an error
| data_size   | u32        | Length in bytes of the valid data in the data field
| data        | u8[...]    | DER-encoded Runtime alias Certificate

### ECDSA384\_SIGNATURE\_VERIFY

Verifies an ECDSA P-384 signature. The hash to be verified is taken from
Caliptra's SHA384 accelerator peripheral.

Command Code: `0x5349_4756` ("SIGV")

Table: `ECDSA384_SIGNATURE_VERIFY` input arguments

| **Name**     | **Type** | **Description**
| --------     | -------- | ---------------
| chksum       | u32      | Checksum over other input arguments, computed by the caller. Little endian.
| pub\_key\_x  | u8[48]   | X portion of ECDSA verification key
| pub\_key\_y  | u8[48]   | Y portion of ECDSA verification key
| signature\_r | u8[48]   | R portion of signature to verify
| signature\_s | u8[48]   | S portion of signature to verify

Table: `ECDSA384_SIGNATURE_VERIFY` output arguments

| **Name**    | **Type** | **Description**
| --------    | -------- | ---------------
| chksum      | u32      | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32      | Indicates if the command is FIPS approved or an error

### STASH\_MEASUREMENT

Make a measurement into the DPE default context. This command is intendend for
callers who update infrequently and cannot tolerate a changing DPE API surface.

* Call the DPE DeriveChild command with the DefaultContext in the locality of
  the PL0 PAUSER.
* Extend the measurement into PCR31 (`PCR_ID_STASH_MEASUREMENT`).

Command Code: `0x4D45_4153` ("MEAS")

Table: `STASH_MEASUREMENT` input arguments

| **Name**     | **Type** | **Description**
| --------     | -------- | ---------------
| chksum       | u32      | Checksum over other input arguments, computed by the caller. Little endian.
| metadata     | u8[4]    | 4-byte measurement identifier.
| measurement  | u8[48]   | Data to measure into DPE.
| context      | u8[48]   | Context field for `svn`, e.g. a hash of the public key that authenticated the SVN.
| svn          | u32      | SVN passed to to DPE to be used in derive child.


Table: `STASH_MEASUREMENT` output arguments

| **Name**    | **Type** | **Description**
| --------    | -------- | ---------------
| chksum      | u32      | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32      | Indicates if the command is FIPS approved or an error
| dpe\_result | u32      | Result code of DPE DeriveChild command. Little endian.

### DISABLE\_ATTESTATION

Disable attestation by erasing the CDI and DICE key. This command is intended
for callers who update infrequently and cannot tolerate a changing DPE API
surface, and is intended for situations where Caliptra firmware cannot be loaded
and the SoC must proceed with boot.

Upon receipt of this command, Caliptra's current CDI is replaced with zeroes,
and the associated DICE key is re-derived from the zeroed CDI.

This command is intended to allow the SoC to continue booting for diagnostic
and error reporting. All attestations produced in this mode are expected to
fail certificate chain validation. Caliptra MUST undergo a cold reset in order
to re-enable attestation.

Command Code: `0x4453_424C` ("DSBL")

Table: `DISABLE_ATTESTATION` input arguments

| **Name**  | **Type**      | **Description**
| --------  | --------      | ---------------
| chksum    | u32           | Checksum over other input arguments, computed by the caller. Little endian.

Table: `DISABLE_ATTESTATION` output arguments

| **Name**    | **Type** | **Description**
| --------    | -------- | ---------------
| chksum      | u32      | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32      | Indicates if the command is FIPS approved or an error

### INVOKE\_DPE\_COMMAND

Invoke a serialized DPE command.

Command Code: `0x4450_4543` ("DPEC")

Table: `INVOKE_DPE_COMMAND` input arguments

| **Name**     | **Type**      | **Description**
| --------     | --------      | ---------------
| chksum       | u32           | Checksum over other input arguments, computed by the caller. Little endian.
| data_size    | u32           | Length in bytes of the valid data in the data field
| data         | u8[...]       | DPE command structure as defined in the DPE iRoT profile


Table: `INVOKE_DPE_COMMAND` output arguments

| **Name**    | **Type**      | **Description**
| --------    | --------      | ---------------
| chksum      | u32           | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32           | Indicates if the command is FIPS approved or an error
| data_size   | u32           | Length in bytes of the valid data in the data field
| data        | u8[...]       | DPE response structure as defined in the DPE iRoT profile.

### QUOTE\_PCRS

Generate a signed quote over all Caliptra hardware PCRs using the Caliptra PCR quoting key.
All PCR values are hashed together with the nonce to produce the quote.

Command Code: `0x5043_5251` ("PCRQ")

Table: `QUOTE_PCRS` input arguments

| **Name**     | **Type**      | **Description**
| --------     | --------      | ---------------
| chksum       | u32           | Checksum over other input arguments, computed by the caller. Little endian.
| nonce        | u8[32]        | Caller-supplied nonce to be included in signed data

Table: `QUOTE_PCRS` output arguments

PcrValue is defined as u8[48]

| **Name**     | **Type**     | **Description**
| --------     | --------     | ---------------
| chksum       | u32          | Checksum over other output arguments, computed by Caliptra. Little endian.
| PCRs         | PcrValue[32] | Values of all PCRs
| nonce        | u8[32]       | Return the nonce used as input for convenience
| digest       | u8[48]       | Return the digest over the PCR values and the nonce
| reset\_ctrs  | u32[32]      | Reset counters for all PCRs
| signature\_r | u8[48]       | R portion of the signature over the PCR quote.
| signature\_s | u8[48]       | S portion of the signature over the PCR quote.

### EXTEND\_PCR

Extend a Caliptra hardware PCR

Command Code: `0x5043_5245` ("PCRE")

Table: `EXTEND_PCR` input arguments

| **Name**     | **Type**      | **Description**
| --------     | --------      | ---------------
| chksum       | u32           | Checksum over other input arguments, computed by the caller. Little endian.
| index        | u32           | Index of the PCR to extend
| value        | u8[..]        | Value to extend into the PCR at `index`


`EXTEND_PCR` returns no output arguments.

Note that extensions made into Caliptra's PCRs are _not_ appended to Caliptra's internal PCR log.

### GET\_PCR\_LOG

Get Caliptra's internal PCR log

Command Code: `0x504C_4F47` ("PLOG")

Table: `GET_PCR_LOG` input arguments

| **Name**  | **Type**      | **Description**
| --------  | --------      | ---------------
| chksum    | u32           | Checksum over other input arguments, computed by the caller. Little endian.

Table: `GET_PCR_LOG` output arguments

| **Name**    | **Type**   | **Description**
| --------    | --------   | ---------------
| chksum      | u32        | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32        | Indicates if the command is FIPS approved or an error
| data_size   | u32        | Length in bytes of the valid data in the data field
| data        | u8[...]    | Internal PCR event log

See [pcr_log.rs](../drivers/src/pcr_log.rs) for the format of the log.

Note: the log contents reflect PCR extensions made autonomously by Caliptra during boot. The log contents
are not preserved across cold or update resets. Callers who wish to verify PCRs that are autonomously
extended during update reset should cache the log before triggering an update reset.

### INCREMENT\_PCR\_RESET\_COUNTER

Increment the reset counter for a PCR

Command Code: `0x5043_5252` ("PCRR")

Table: `INCREMENT_PCR_RESET_COUNTER` input arguments

| **Name**     | **Type**      | **Description**
| --------     | --------      | ---------------
| chksum       | u32           | Checksum over other input arguments, computed by the caller. Little endian.
| index        | u32           | Index of the PCR for which to increment the reset counter

`INCREMENT_PCR_RESET_COUNTER` returns no output arguments.

### DPE\_TAG\_TCI

Associates a unique tag with a DPE context

Command Code: `0x5451_4754` ("TAGT")

Table: `DPE_TAG_TCI` input arguments

| **Name**     | **Type**      | **Description**
| --------     | --------      | ---------------
| chksum       | u32           | Checksum over other input arguments, computed by the caller. Little endian.
| handle       | u8[16]        | DPE context handle 
| tag          | u32           | A unique tag which the handle will be associated with 

Table: `DPE_TAG_TCI` output arguments

| **Name**    | **Type** | **Description**
| --------    | -------- | ---------------
| chksum      | u32      | Checksum over other output arguments, computed by Caliptra. Little endian.
| fips_status | u32      | Indicates if the command is FIPS approved or an error

### DPE\_GET\_TAGGED\_TCI

Retrieves the TCI measurements corresponding to the tagged DPE context 

Command Code: `0x4754_4744` ("GTGD")

Table: `DPE_GET_TAGGED_TCI` input arguments

| **Name**     | **Type**      | **Description**
| --------     | --------      | ---------------
| chksum       | u32           | Checksum over other input arguments, computed by the caller. Little endian.
| tag          | u32           | A unique tag corresponding to a DPE context

Table: `DPE_GET_TAGGED_TCI` output arguments

| **Name**          | **Type**       | **Description**
| --------          | --------       | ---------------
| chksum            | u32            | Checksum over other input arguments, computed by the caller. Little endian.
| tci_cumulative    | u8[48]         | Hash of all the input data provided to the context
| tci_current       | u8[48]         | Most recent measurement made into the context

## Checksum

For every command, the request and response feature a checksum. This mitigates
glitches between clients and Caliptra.

The Checksum is a little-endian 32-bit value, defined as:

```
0 - (SUM(command code bytes) + SUM(request/response bytes))
```

The sum of all bytes in a request/response body, and command code, should be
zero.

If Caliptra detects an invalid Checksum in input parameters, it will return
`BAD_CHKSUM` as the result.

Caliptra will also compute a Checksum over all responses and write it to the
chksum field.

## FIPS Status

For every command, the firmware will respond with FIPS status of FIPS approved. There is
currently no use-case for any other responses or error values.

Table: FIPS status codes:

| **Name**         | **Value**                   | Description
| -------          | -----                       | -----------
| `FIPS_APPROVED`  | `0x0000_0000`               | Status of command is FIPS approved
| `RESERVED`       | `0x0000_0001 - 0xFFFF_FFFF` | Other values reservered, will not be sent by Caliptra

## Runtime Firmware Updates

Caliptra Runtime firmware accepts impactless updates which will update
Caliptra’s firmware without resetting other cores in the SoC.

### Applying Updates

A Runtime Firmware update is triggered by the `CALIPTRA_FW_LOAD` command. Upon
receiving this command, Runtime Firmware will:

1. Write-lock mailbox
1. Invoke “Impactless Reset”

Once Impactless Reset has been invoked, FMC will load the hash of the image
from the verified Manifest into the necessary PCRs:

1. Runtime Journey PCR
1. Runtime Latest PCR

If ROM validation of the image fails,

* ROM SHALL not clear the "Runtime Latest" PCR. It SHALL still re-lock this
  PCR with the existing value.
* FMC SHALL NOT extend either of the Runtime PCRs.

### Boot Process After Update

After an Impactless Update has been applied, the new Runtime Firmware will be
able to sample a register to determine it has undergone an Impactless Reset. In
this case, the new Runtime Firmware must:

1. Validate DPE state in SRAM
    1. Ensure TCI tree is well-formed
    1. All nodes chain to the root (TYPE = RTJM, “Internal TCI” flag is set)
1. Verify that the “Latest TCI” field of the TCI Node which contains the
   Runtime Journey PCR (TYPE = RTJM, “Internal TCI” flag is set) matches the
   “Latest” Runtime PCR value from PCRX
    1. Ensure `SHA384_HASH(0x00..00, TCI from SRAM) == RT_FW_JOURNEY_PCR`
1. Check that retired and inactive contexts do not have tags
1. If any validations fail, runtime firmware will execute the
   `DISABLE_ATTESTATION` command.

## DICE Protection Environment (DPE)

Caliptra Runtime Firmware SHALL implement a profile of the DICE Protection
Environment (DPE) API.

### PAUSER Privilege Levels

Caliptra models PAUSER callers to its mailbox as having 1 of 2 privilege levels:

* PL0 - High Privilege. Only 1 PAUSER in the SoC may be at PL0. The PL0 PAUSER
  is denoted in the signed Caliptra firmware image. The PL0 PAUSER may call any
  supported DPE commands. Only PL0 can use the CertifyKey command. Success of the
  CertifyKey command signifies to the caller that it is at PL0. Only PL0 can use 
  the POPULATE_IDEV_CERT mailbox command. 
* PL1 - Restricted Privilege. All other PAUSERs in the SoC are PL1. Caliptra
  SHALL fail any calls to the DPE CertifyKey command by PL1 callers.
  PL1 callers should use the CertifyCsr command instead.

#### PAUSER Privilege Level Active Context Limits

Each active context in DPE is activated from either PL0 or PL1 through the
InvokeDpe mailbox command calling the DeriveChild or InitializeContext DPE
commands. However, a caller could easily exhaust space in DPE's context array
by repeatedly calling the aforementioned DPE commands with certain flags set.

To prevent against this, we establish active context limits for each PAUSER
privilege level:

* PL0 - 8 active contexts
* PL1 - 16 active contexts

If a DPE command were to activate a new context such that the total number of
active contexts in a privilege level is above its active context limit, the
InvokeDpe command should fail.

Further, it is not allowed for PL1 to call DeriveChild with the intent of 
changing locality to PL0's locality, since this would increase the number 
of active contexts in PL0's locality, and hence allow PL1 to DOS PL0.

### DPE Profile Implementation

The DPE iRoT Profile leaves some choices up to implementers. This section
details specific requirements for the Caliptra DPE implementation.

| Name                       | Value                          | Description
| ----                       | -----                          | -----------
| Profile Variant            | `DPE_PROFILE_IROT_P384_SHA384` | The profile variant that Caliptra implements.
| KDF                        | SP800-108 HMAC-CTR             | KDF to use for CDI (tcg.derive.kdf-sha384) and asymmetric key (tcg.derive.kdf-sha384-p384) derivation.
| Simulation Context Support | Yes                            | Whether Caliptra implements the optional Simulation Contexts feature
| Supports ExtendTci         | Yes                            | Whether Caliptra implements the optional ExtendTci command
| Supports Auto Init         | Yes                            | Whether Caliptra will automatically initialize the default DPE context.
| Supports Rotate Context    | Yes                            | Whether Caliptra supports the optional RotateContextHandle command.
| CertifyKey Alias Key       | Caliptra Runtime Alias Key     | The key that will be used to sign certificates produced by the DPE CertifyKey command.

### Supported DPE Commands

Caliptra DPE supports the following commands

* GetProfile
* InitializeContext
* DeriveChild
* CertifyKey
  * Caliptra DPE supports two formats for CertifyKey: X.509 and PKCS#10 CSR.
    X.509 is only available to PL0 PAUSERs.
* Sign
* RotateContextHandle
* DestroyContext
* GetCertificateChain

In addition, Caliptra supports the following profile-defined command:

* ExtendTci: Extend a TCI measurement made by DeriveChild to provide additional
             measurement data.

### DPE State Atomicity

This implementation guarantees that no internal DPE state is changed if a
command fails for any reason. This includes Context Handle rotation; single-use
context handles are not rotated if a command fails.

On failure, DPE will only return a command header, with no additional
command-specific response parameters. This is in line with the CBOR-based
main DPE spec, which does not return a response payload on failure.

### Initializing DPE

Caliptra Runtime firmware is responsible for initializing DPE’s Default Context.

* Runtime Firmware SHALL initialize the Default Context in “internal-cdi” mode.
* Call DeriveChild to measure the Caliptra Journey PCR
* INPUT\_DATA = PCRX (RT journey PCR)
* TYPE = “RTJM”
* CONTEXT\_HANDLE = Default context
* Set flag in the TCI Node that this node was created by the DPE implementation.
  This will be used to set the VENDOR\_INFO field in TcbInfo to “VNDR”.

When initializing DPE, Runtime Firmware shall also add each of ROM's stashed 
measurements to DPE. To do so, it should call DeriveChild for each measurement
in the measurement log. 

*Note: the Runtime CDI (from KeyVault) will be used as-needed and will not be
accessed during initialization.*

### CDI Derivation

The DPE Sign and CertifyKey commands derive an asymmetric key for that handle.

DPE will first collect measurements and concatenate them in a byte buffer
`MEASUREMENT_DATA`:

* LABEL parameter passed to Sign or CertifyKey.
* The `TCI_NODE_DATA` structures in the path from the current TCI node to the
  root, inclusive, starting with the current node.

To derive a CDI for a given context, DPE shall use KeyVault hardware with the
following inputs:

* CDI = Runtime Firmware CDI (from KeyVault)
* Label = LABEL parameter provided to Sign or CertifyKey
* Context = `MEASUREMENT_DATA`

The CDI shall be loaded into KeyVault slot 0.

### Leaf Key Derivation

To derive an asymmetric key for Sign and CertifyKey, RT will

* Derive an ECC P384 keypair from KV slot 0 CDI into KV slot 1
* For CertifyKey: Request the public key
* For Sign: Sign passed data
* Erase KeyVault slots 0 and 1

### Internal Representation of TCI Nodes

| Byte Offset | Bits  | Name           | Description
| ----------- | ----- | ------------   | -------------
| 0x02        | 15:0  | Parent Index   | Index of the TCI node that is the parent of this node. 0xFF if this node is the root.
| 0x04        | 159:0 | Context Handle | DPE context handle referring to this node
| 0x18        | 31    | Internal TCI   | This TCI was measured by Runtime Firmware itself
|             | 30:0  | Reserved       | Reserved flag bits
| 0x1C        | 383:0 | Latest TCI     | The latest `INPUT_DATA` extended into this TCI by ExtendTci or DeriveChild

Table: `TCI_NODE_DATA` for `DPE_PROFILE_IROT_P384_SHA384`

| **Byte Offset** | **Bits** | **Name**         | **Description**
| -----           | ----     | ---------------- | -----------------------------------------------------
| 0x00            | 383:0    | `TCI_CURRENT`    | "Current" TCI measurement value
| 0x30            | 383:0    | `TCI_CUMULATIVE` | TCI measurement value
| 0x60            | 31:0     | `TYPE`           | `TYPE` parameter to the DeriveChild call which created this node
| 0x64            | 31:0     | `LOCALITY`       | `TARGET_LOCALITY` parameter to the DeriveChild call which created this node (PAUSER)

### Certificate Generation

The DPE Runtime Alias Key SHALL sign DPE leaf certificates and CSRs.

The DPE `GET_CERTIFICATE_CHAIN` command shall return the following certificates:

* IDevID (Optionally added by the SoC via POPULATE_IDEV_CERT)
* LDevID
* FMC Alias
* Runtime Alias

### DPE Leaf Certificate Definition

| Field                          | Sub Field   | Value
| -------------                  | ---------   | ---------
| Version                        | v3          | 2
| Serial Number                  |             | First 20 bytes of sha256 hash of DPE Alias public key
| Issuer Name                    | CN          | Caliptra Runtime Alias
|                                | serialNumber | First 20 bytes of sha384 hash of Runtime Alias public key
| Validity                       | notBefore   | notBefore from firmware manifest
|                                | notAfter    | notAfter from firmware manifest
| Subject Name                   | CN          | Caliptra DPE Leaf
|                                | serialNumber | SHA384 hash of Subject public key
| Subject Public Key Info        | Algorithm   | ecdsa-with-SHA384
|                                | Parameters  | Named Curve = prime384v1
|                                | Public Key  | DPE Alias Public Key value
| Signature Algorithm Identifier | Algorithm   | ecdsa-with-SHA384
|                                | Parameters  | Named Curve = prime384v1
| Signature Value                |             | Digital signature for the certificate
| KeyUsage                       | keyCertSign | 1
| Basic Constraints              | CA          | False
| Policy OIDs                    |             | id-tcg-kp-attestLoc
| tcg-dice-MultiTcbInfo\*        | Vendor      | {Vendor-defined}
|                                | Model       | Caliptra
|                                | SVN         | 1
|                                | FWIDs       | [0] "Journey" TCI Value
|                                |             | [1] "Current" TCI Value. Latest `INPUT_DATA` made by DeriveChild or ExtendTci.
|                                | Type        | 4-byte TYPE field of TCI node
|                                | VendorInfo  | Locality of the caller (analog for PAUSER)

\*MultiTcbInfo ontains one TcbInfo for each TCI Node in the path from the
current TCI Node to the root. Max of 24.

# Opens

Needs clarification or more details:

* Describe mailbox flow for commands which need to send data which exceeds the
  mailbox size
* This specification should fully enumerate how runtime firmware uses shared
  hardware resources. See https://github.com/chipsalliance/caliptra-sw/issues/17
