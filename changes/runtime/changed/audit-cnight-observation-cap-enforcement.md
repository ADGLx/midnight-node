#runtime
# Enforce per-address registration cap in cnight-observation

`pallet-cnight-observation::handle_registration` previously appended to the
`Mappings` storage map without checking the per-address cap declared by the
mock-runtime constants and referenced (but never enforced) by the dead
`Error::MaxRegistrationsExceeded` variant. A single Cardano reward address
could therefore grow `Mappings::<T>::get(addr)` without bound through repeated
registration UTXOs. The cap is now an associated type
`Config::MaxRegistrationsPerCardanoAddress: Get<u32>` (bound to `100` in the
runtime and both mocks) and is enforced as a writer invariant inside
`handle_registration`: over-cap registrations are dropped with a new
`Event::MappingCapped(MappingEntry)` event and a `log::warn!` diagnostic, and
the surrounding `process_tokens` inherent batch continues processing
sibling UTXOs. `Mappings` storage type is unchanged (still `Vec<MappingEntry>`)
so no `OnRuntimeUpgrade` migration is required; the now-dead
`Error::MaxRegistrationsExceeded` variant is removed.

PR: https://github.com/midnightntwrk/midnight-node/pull/1488
Ticket: https://midnight-ntwrk.atlassian.net/browse/PM-19974
