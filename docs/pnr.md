# Pointer Name Resolver (PNR) [EXPERIMENTAL]

The Pointer Name Resolver (PNR) is a first-come, first-served name resolution service for the Autonomi Network. It allows users to register human-readable names that resolve to network addresses (like XOR addresses for archives or files).

> **Important:** The PNR system is currently experimental. The resolver key and implementation details are subject to change during initial testing.

## How it Works

PNR uses a shared key to store and update pointers. These pointers create a chain that ultimately resolves to a target address.

### Registering a Name
To create a name, a client sets the `x-data-key` header to `resolver` and creates a pointer pointing to their target address. This uses a shared resolver key known to AntTP instances.

### Transferring Ownership
Ownership can be transferred by creating a pointer chain. The current owner updates their target pointer to point to a new owner's pointer and sets the 'counter' to the maximum value (`18,446,744,073,709,551,614` via REST API). Once the counter reaches the `u64` maximum, previous pointers in the chain can no longer be modified, effectively transferring control.

`PNR Shared Key Pointer (max counter) ⇒ PNR Old User Key Pointer (max counter) ⇒ PNR Current User Key Pointer`

## Performance and Caching

As the owner chain grows, the time to resolve a name would naturally increase. To mitigate this, AntTP aggressively caches pointers. After the initial retrieval, the impact of a long pointer chain is virtually unnoticeable.

## Advanced Use Cases

Pointer chains also allow for name "renting" or management without full transfer. A manager can point to a consumer's pointer without setting the maximum counter, retaining the ability to re-point it later.

In the future, PNR may be extended to support more complex features, such as sub-names managed via an immutable lookup table.
