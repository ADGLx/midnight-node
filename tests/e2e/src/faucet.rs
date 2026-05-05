use crate::api::cardano::CardanoClient;
use crate::config::OgmiosClientSettings;
use ogmios_client::types::OgmiosUtxo;
use tokio::sync::Mutex;
use whisky::Asset;

const MAX_INPUT_UTXOS: usize = 20;

pub struct FaucetManager {
    pub ogmios_settings: OgmiosClientSettings,
    pub faucet: CardanoClient,
    locked_utxos: Mutex<Vec<OgmiosUtxo>>,
    utxo_lock: Mutex<()>,
}

impl FaucetManager {
    pub async fn new(ogmios_settings: OgmiosClientSettings, faucet: CardanoClient) -> Self {
        FaucetManager {
            ogmios_settings,
            faucet,
            locked_utxos: Mutex::new(Vec::new()),
            utxo_lock: Mutex::new(()),
        }
    }

    pub async fn request_tokens(&self, address: &str, lovelace: u64) -> OgmiosUtxo {
        let tx_ins = self
            .lock_utxos(lovelace)
            .await
            .expect("Failed to lock UTXOs for faucet request");
        let assets = vec![Asset::new_from_str("lovelace", &lovelace.to_string())];
        self.faucet
            .fund_wallet(&tx_ins, address, assets)
            .await
            .expect("Failed to request tokens from faucet")
    }

    fn utxo(u: &OgmiosUtxo) -> ([u8; 32], u16) {
        (u.transaction.id, u.index)
    }

    async fn lock_utxos(&self, lovelace: u64) -> Option<Vec<OgmiosUtxo>> {
        // 2 ADA covers the tx fee (~0.25 ADA) and keeps the change UTXO above
        // Cardano's min-UTXO threshold (~1 ADA without native tokens).
        self.lock_utxos_with_buffer(lovelace, 2_000_000).await
    }

    async fn lock_utxos_with_buffer(
        &self,
        lovelace: u64,
        buffer: u64,
    ) -> Option<Vec<OgmiosUtxo>> {
        let _guard = self.utxo_lock.lock().await;
        let expected = lovelace + buffer;

        let locked_ids: Vec<_> = self
            .locked_utxos
            .lock()
            .await
            .iter()
            .map(Self::utxo)
            .collect();

        let mut available: Vec<_> = self
            .faucet
            .utxos()
            .await
            .into_iter()
            .filter(|u| !locked_ids.contains(&Self::utxo(u)))
            .collect();

        // Smallest-first: cleans up fragmented change UTXOs left behind by previous sends.
        available.sort_by_key(|u| u.value.lovelace);

        let mut selected = Vec::new();
        let mut sum: u64 = 0;
        for u in available {
            if selected.len() >= MAX_INPUT_UTXOS {
                break;
            }
            sum = sum.saturating_add(u.value.lovelace);
            selected.push(u);
            if sum >= expected {
                break;
            }
        }

        if sum < expected {
            println!(
                "Faucet exhausted or too fragmented: {} unlocked UTXOs sum to {} lovelace, need {}",
                selected.len(),
                sum,
                expected,
            );
            return None;
        }

        self.locked_utxos
            .lock()
            .await
            .extend(selected.iter().cloned());
        Some(selected)
    }
}
