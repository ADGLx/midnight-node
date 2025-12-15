use crate::config::NodeClientSettings;
use bip39::{Language, Mnemonic, MnemonicType};
use blake2::digest::{Update, VariableOutput};
use blake2::Blake2bVar;
use hex::ToHex;
use midnight_node_ledger_helpers::mn_ledger::dust;
use midnight_node_ledger_helpers::{serialize_untagged, DefaultDB, DustWallet, IntoWalletAddress, WalletSeed};
use midnight_node_ledger_helpers::{UnshieldedWallet, WalletAddress};
use midnight_node_metadata::midnight_metadata_latest::c_night_observation::storage::types::utxo_owners::UtxoOwners;
use midnight_node_metadata::midnight_metadata_latest::federated_authority_observation::events::{CouncilMembersReset, TechnicalCommitteeMembersReset};
use midnight_node_metadata::midnight_metadata_latest::runtime_types::midnight_primitives_cnight_observation::ObservedUtxo;
use midnight_node_metadata::midnight_metadata_latest::{
	self as mn_meta,
	c_night_observation::{self}
	,
};
use std::time::Duration;
use subxt::blocks::ExtrinsicEvents;
use subxt::utils::H256;
use subxt::{OnlineClient, SubstrateConfig};
use tokio::time::{sleep, timeout, Instant};

pub struct MidnightClient {
    pub online_client: OnlineClient<SubstrateConfig>,
}

impl MidnightClient {
    pub async fn new(node_settings: NodeClientSettings) -> Self {
        let online_client =
            OnlineClient::<SubstrateConfig>::from_insecure_url(node_settings.base_url)
                .await
                .expect("Failed to initialize client");
        Self { online_client }
    }

    pub fn new_seed() -> WalletSeed {
        let mnemonic: Mnemonic = Mnemonic::new(MnemonicType::Words24, Language::English);
        println!("New mnemonic: {}", mnemonic);
        let phrase = mnemonic.phrase().to_string();

        println!(
            "Generated mnemonic phrase for new Midnight wallet: {}",
            phrase
        );

        let mnemonic_seed: WalletSeed = phrase.parse().unwrap();

        println!(
            "Derived Midnight wallet seed from mnemonic: {}",
            hex::encode(mnemonic_seed.as_bytes())
        );
        let unshielded_wallet = UnshieldedWallet::default(mnemonic_seed);
        println!(
            "Derived Midnight unshielded address: {}",
            unshielded_wallet.address("preview").to_bech32()
        );
        // let wallet_address: WalletAddress = unshielded_wallet.user_address.into();
        // println!("Derived Midnight wallet address: {:?}", wallet_address);
        // let wallet_seed = WalletSeed::try_from_mnemonic(&phrase);
        // println!(
        //     "Midnight wallet seed: {}",
        //     hex::encode(wallet_seed.as_bytes())
        // );
        mnemonic_seed
        // let seed_bytes: [u8; 32] = rand::random();
        // println!("Midnight seed: {}", hex::encode(seed_bytes));
        // WalletSeed::from(seed_bytes)
    }

    pub fn new_dust_hex(wallet_seed: WalletSeed) -> String {
        let dust_wallet = DustWallet::<DefaultDB>::default(wallet_seed, None);
        let dust_public = dust_wallet.public_key;

        let dust_address = dust_wallet.address("preview");
        println!("dust pb key: {:?}", dust_address.to_bech32());

        // let dust_wallet_from_address: DustWallet<DefaultDB> = dust_address.try_into()
        let dust_wallet_from_address: DustWallet<DefaultDB> =
            DustWallet::try_from(&dust_address).unwrap();
        println!(
            "Re-derived dust pb key from address: {:?}",
            dust_wallet_from_address.public_key
        );
        let dust_bytes2 = serialize_untagged(&dust_wallet_from_address.public_key).unwrap();
        println!(
            "RADOO: dust wallet from address hex: {}",
            dust_bytes2.encode_hex::<String>()
        );

        let mut dust_bytes = serialize_untagged(&dust_public).unwrap();
        if dust_bytes.len() == 32 {
            dust_bytes.push(0);
        }
        let dust_public_hex = dust_bytes.encode_hex::<String>();
        println!("Dust public key hex: {}", dust_public_hex);
        dust_public_hex
    }

    pub async fn subscribe_to_cnight_observation_events(
        &self,
        tx_id: &[u8],
    ) -> Result<ExtrinsicEvents<SubstrateConfig>, Box<dyn std::error::Error>> {
        println!(
            "Subscribing for cNIGHT observation extrinsic with tx_id: 0x{}",
            hex::encode(tx_id)
        );
        let mut blocks_sub = self.online_client.blocks().subscribe_finalized().await?;

        let inner = async {
            while let Some(block_result) = blocks_sub.next().await {
                let block = block_result?;

                let block_number = block.header().number;
                println!("Finalized block #{}", block_number);

                let extrinsic = block.extrinsics().await?;

                for ext in extrinsic.iter() {
                    let Ok(decoded) = ext.as_root_extrinsic::<mn_meta::Call>() else {
                        continue;
                    };

                    let Some(utxos) = MidnightClient::extract_process_tokens_utxos(&decoded) else {
                        continue;
                    };

                    println!(
                        "  NativeTokenObservation::process_tokens called with {} UTXOs",
                        utxos.len()
                    );

                    if utxos.is_empty() {
                        continue;
                    }

                    if utxos.iter().any(|u| u.header.tx_hash.0 == tx_id) {
                        println!(
                            "*** Found UTXO with matching registration tx hash: 0x{} ***",
                            hex::encode(tx_id)
                        );
                        let events = ext.events().await?;
                        return Ok(events);
                    } else {
                        for u in utxos {
                            let seen = u.header.tx_hash.0;
                            println!(
                                "Tx hash 0x{} does not match expected registration tx hash 0x{}",
                                hex::encode(seen),
                                hex::encode(tx_id)
                            );
                        }
                    }
                }
            }
            Err("Did not find registration event".into())
        };

        timeout(Duration::from_secs(60), inner)
            .await
            .unwrap_or_else(|_| Err("Timeout waiting for registration event".into()))
    }

    pub fn calculate_nonce(prefix: &[u8], tx_hash: [u8; 32], tx_index: u16) -> String {
        let mut hasher = Blake2bVar::new(32).expect("valid output size");

        hasher.update(prefix);
        hasher.update(&tx_hash);
        hasher.update(&tx_index.to_be_bytes());

        let mut out = [0u8; 32];
        hasher
            .finalize_variable(&mut out)
            .expect("finalize succeeds");
        hex::encode(out)
    }

    fn extract_process_tokens_utxos(call: &mn_meta::Call) -> Option<&Vec<ObservedUtxo>> {
        match call {
            mn_meta::Call::CNightObservation(c_night_observation::Call::process_tokens {
                utxos,
                ..
            }) => Some(utxos),
            _ => None,
        }
    }

    pub async fn query_night_utxo_owners(
        &self,
        utxo: String,
    ) -> Result<Option<UtxoOwners>, Box<dyn std::error::Error>> {
        let nonce = hex::decode(&utxo).unwrap();
        let storage_address = mn_meta::storage()
            .c_night_observation()
            .utxo_owners(H256(nonce.try_into().unwrap()));

        let owners = self
            .online_client
            .storage()
            .at_latest()
            .await?
            .fetch(&storage_address)
            .await?;

        Ok(owners)
    }

    pub async fn poll_utxo_owners_until_change(
        &self,
        utxo: String,
        initial_value: Option<UtxoOwners>,
        timeout_secs: u64,
        poll_interval_ms: u64,
    ) -> Result<Option<UtxoOwners>, Box<dyn std::error::Error>> {
        let start = Instant::now();
        loop {
            let current_value = self.query_night_utxo_owners(utxo.clone()).await?;
            if current_value.as_ref().map(|v| v.0.0.clone())
                != initial_value.as_ref().map(|v| v.0.0.clone())
            {
                println!("UtxoOwners storage changed: {:?}", current_value);
                return Ok(current_value);
            }
            if start.elapsed() > Duration::from_secs(timeout_secs) {
                println!("Timeout reached without change");
                return Ok(current_value);
            }
            sleep(Duration::from_millis(poll_interval_ms)).await;
        }
    }

    pub async fn subscribe_to_federated_authority_events(
        &self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Subscribing to federated authority observation events");

        let mut blocks_sub = self.online_client.blocks().subscribe_finalized().await?;

        let result = timeout(Duration::from_secs(120), async {
            while let Some(block) = blocks_sub.next().await {
                let block = block?;
                let block_number = block.header().number;
                println!("Checking block #{block_number} for federated authority events");

                let events = block.events().await?;

                // Check for CouncilMembersReset event
                let council_reset = events.find::<CouncilMembersReset>().flatten().next();

                // Check for TechnicalCommitteeMembersReset event
                let tech_committee_reset = events
                    .find::<TechnicalCommitteeMembersReset>()
                    .flatten()
                    .next();

                if let Some(event) = &council_reset {
                    println!(
                        "✓ Found CouncilMembersReset event with {} members",
                        event.members.0.len()
                    );
                }
                if let Some(event) = &tech_committee_reset {
                    println!(
                        "✓ Found TechnicalCommitteeMembersReset event with {} members",
                        event.members.0.len()
                    );
                }

                if council_reset.is_some() && tech_committee_reset.is_some() {
                    return Ok(());
                }
            }
            Err("Did not find all federated authority events".into())
        })
        .await;

        result.unwrap_or_else(|_| Err("Timeout waiting for federated authority events".into()))
    }
}
