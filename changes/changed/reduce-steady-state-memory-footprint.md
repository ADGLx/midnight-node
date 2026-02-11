#node #memory
# Reduce steady-state memory footprint on fully synced validators

Heaptrack profiling and production monitoring show that fully synced validators
exhibit linear memory growth over time, eventually approaching pod memory limits.
The previous fix (storage_cache_size bounded LRU) addressed the dominant allocator
during chain sync, but three other memory sources continue growing on synced nodes:

1. **Substrate trie cache (1 GiB → 256 MiB):** The trie cache fills gradually and
   never shrinks. At 1 GiB it consumed a fixed ~1 GiB on every node. 256 MiB is
   sufficient for 6-second block validators.

2. **WASM runtime instances (8 → 2):** Each pooled instance reserves 128 MiB of
   heap that is allocated on demand and never released. Default of 8 = up to 1 GiB.
   Validators only need 1–2 concurrent instances.

3. **Transaction validation caches (add 5-minute TTI):** The moka caches for
   VerifiedTransaction objects had no time-based eviction. On low-traffic networks
   stale entries (50–200 KiB each, containing ZK proof data) persisted indefinitely.
   Adding time_to_idle eviction prevents accumulation during quiet periods.

Combined savings: ~1.5 GiB reduction in steady-state memory per validator.
