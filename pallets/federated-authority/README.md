# pallet-federated-authority

Cross-collective governance mechanism requiring multi-body approval for privileged operations.

## Overview

The `federated_authority` [pallet](https://docs.midnight.network/learn/glossary#pallet) implements a [federated authority](https://docs.midnight.network/learn/glossary#federated-authority) system where multiple distinct on-chain authority bodies must collectively approve a motion before it can be executed with elevated `Root` privileges. This creates a final checkpoint requiring consensus from multiple governance groups before any critical action can be executed.

The [pallet](https://docs.midnight.network/learn/glossary#pallet) is configurable to define:
- How many authority bodies participate in the federation
- Which collectives or governance groups those bodies represent
- The approval thresholds and voting mechanisms for each body
- The number of approvals required to dispatch a motion
- The lifetime of a motion before it expires

## API Specification

### Dispatchables

- [**`motion_approve`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L125) - Signal approval of a call from an authority body
- [**`motion_close`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L180) - Finalize an approved or expired motion
- [**`motion_revoke`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L230) - Withdraw approval before execution

### Storage Items

- [**`Motions`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L80) - Pending motions awaiting approval
- [**`MotionApprovals`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L85) - Approval status per body

### Events

- [**`MotionCreated`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L92) - New motion created on first approval
- [**`MotionApproved`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L94) - Authority body approved a motion
- [**`MotionRevoked`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L96) - Authority body revoked approval
- [**`MotionExecuted`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L98) - Motion executed with Root privileges
- [**`MotionExpired`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L100) - Motion expired without sufficient approvals

### Errors

- [**`MotionNotFound`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L105) - Referenced motion doesn't exist
- [**`AlreadyApproved`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L107) - Body already approved this motion
- [**`NotApproved`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L109) - Cannot revoke non-existent approval
- [**`MotionExpired`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L111) - Motion lifetime exceeded
- [**`InsufficientApprovals`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L113) - Not enough approvals to execute

### Config Trait

- [**`AuthorityBodies`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L55) - List of authority body identifiers
- [**`RequiredApprovals`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L58) - Number of approvals needed for execution
- [**`MotionDuration`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L61) - Block count before motion expires
- [**`RuntimeCall`**](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L64) - The call type that can be executed

## Motion Lifecycle

### 1. Initiating a Motion

A motion is not created directly. Instead, one of the authority bodies signals its approval of a particular call:

- The body conducts its own internal decision-making process (e.g., through a collective vote)
- If its rules are satisfied, it dispatches `motion_approve` with the target call
- On the first approval, the [pallet](https://docs.midnight.network/learn/glossary#pallet) creates a motion entry in storage with an expiration period

### 2. Gathering Approvals

Once recorded, the motion is pending further approvals:

- Each other body must go through its own internal process to approve the exact same call
- If they approve, they also dispatch `motion_approve`, adding their approval to the motion

### 3. Executing or Closing

The `motion_close` [extrinsic](https://docs.midnight.network/learn/glossary#extrinsic) can be called by anyone to finalize a motion. A motion can only be closed if it has either been approved by all required bodies or has expired.

### 4. Revoking an Approval

The `motion_revoke` [extrinsic](https://docs.midnight.network/learn/glossary#extrinsic) allows an authority body to withdraw its approval before execution. If all approvals are revoked, the motion is immediately removed from storage.

## Architecture

```
+-------------------+                       +-------------------+
|     Council       |                       | Technical         |
|   (2/3 approval)  |                       | Committee         |
+--------+----------+                       | (2/3 approval)    |
         |                                  +--------+----------+
         |                                           |
         v                                           v
+--------+-------------------------------------------+----------+
|                      motion_approve()                         |
+---------------------------------------------------------------+
                                   |
                                   v
                    +------------------------------+
                    |     Federated Authority      |
                    |     Pallet                   |
                    |  +-----------------------+   |
                    |  | Motion Storage        |   |
                    |  | - Hash -> Motion      |   |
                    |  | - Approvals tracking  |   |
                    |  +-----------------------+   |
                    +------------------------------+
                                   |
                    (when both bodies have approved)
                                   |
                                   v
                    +------------------------------+
                    |     motion_close()           |
                    |     Execute with Root        |
                    +------------------------------+
```

**Sources**: [[1]](https://github.com/midnightntwrk/midnight-node/blob/main/pallets/federated-authority/src/lib.rs#L125-L280) [[2]](https://github.com/midnightntwrk/midnight-node/blob/main/runtime/src/lib.rs#L916-L954)

## Integration

### Dependencies

- `frame-support` / `frame-system` - [FRAME](https://docs.midnight.network/learn/glossary#frame-framework-for-runtime-aggregation-of-modularized-entities) primitives
- `pallet-collective` - For authority body voting

### Used By

- `runtime` - Governance configuration
- `pallet-federated-authority-observation` - Membership updates

## Testing

```bash
cargo test -p pallet-federated-authority
```

## See Also

- [pallet-federated-authority-observation](../federated-authority-observation/README.md) - Membership sync from Cardano
- [GLOSSARY - Federated Authority](https://docs.midnight.network/learn/glossary#federated-authority) - Term definition
