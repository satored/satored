use anyhow::Result;
use dotenv::dotenv;
use ebx_lib::{
    buffer::Buffer, domain::Domain, header::Header, header_chain::HeaderChain, key_pair::KeyPair,
    merkle_txs::MerkleTxs, pkh::Pkh, priv_key::PrivKey, pub_key::PubKey, tx::Tx,
};
use ebx_mine::db::{
    mine_header::MineHeader, mine_lch::MineLch, mine_merkle_proof::MineMerkleProof,
    mine_tx_parsed::MineTxParsed, mine_tx_raw::MineTxRaw,
};
use env_logger;
use log::{debug, error, info};
use sqlx::mysql::MySqlPool;
use std::{env, error::Error};
use tokio::time::{interval, Duration};

#[allow(dead_code)] // TODO: remove before launch
struct EnvConfig {
    domain: String,
    domain_priv_key: PrivKey,
    domain_key_pair: KeyPair,
    coinbase_pkh: Pkh,
    admin_pub_key: PubKey,
    database_url: String,
}

impl EnvConfig {
    fn new() -> Result<Self, Box<dyn Error>> {
        dotenv().ok();

        let domain = env::var("DOMAIN")?;
        if !Domain::is_valid_domain(&domain) {
            return Err("Invalid domain".into());
        }

        let domain_priv_key_str =
            env::var("DOMAIN_PRIV_KEY").map_err(|_| "Missing domain priv key".to_string())?;
        let domain_priv_key: PrivKey = PrivKey::from_string(&domain_priv_key_str)
            .map_err(|e| format!("Invalid domain priv key: {}", e))?;

        let domain_key_pair: KeyPair = KeyPair::from_priv_key(&domain_priv_key)
            .map_err(|e| format!("Invalid domain key pair: {}", e))?;

        let coinbase_pkh_str =
            env::var("COINBASE_PKH").map_err(|_| "Missing coinbase pkh".to_string())?;
        let coinbase_pkh: Pkh = Pkh::from_string(&coinbase_pkh_str)
            .map_err(|e| format!("Invalid coinbase pkh: {}", e))?;

        let admin_pub_key_str =
            env::var("ADMIN_PUB_KEY").map_err(|_| "Missing admin pub key".to_string())?;
        let admin_pub_key: PubKey = PubKey::from_string(&admin_pub_key_str)
            .map_err(|e| format!("Invalid admin pub key: {}", e))?;

        let database_url =
            env::var("DATABASE_URL").map_err(|_| "Missing database URL".to_string())?;

        Ok(Self {
            domain,
            domain_priv_key,
            domain_key_pair,
            coinbase_pkh,
            admin_pub_key,
            database_url,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let config = EnvConfig::new().unwrap();

    let pool = MySqlPool::connect(&config.database_url)
        .await
        .map_err(|e| anyhow::Error::msg(format!("Failed to connect to database: {}", e)))?;

    let mut longest_chain: HeaderChain = MineLch::get_longest_chain(&pool).await?;
    let mut building_block_num = longest_chain.headers.len();

    let mut interval = interval(Duration::from_secs(1));
    interval.tick().await;

    info!(
        "EBX Mine: {}. Building block: {}.",
        config.domain, building_block_num
    );

    let mut loop_count: u64 = 0;

    'main_loop: loop {
        debug!(
            "Loop count: {}. Building block num: {}.",
            loop_count, building_block_num
        );
        loop_count += 1;

        // 1. Synchronize memory with longest chain from DB
        {
            if longest_chain.headers.len() == 0 {
                longest_chain = MineLch::get_longest_chain(&pool).await?;
            } else {
                let db_chain_tip_id_opt: Option<String> = MineLch::get_chain_tip_id(&pool).await;
                if db_chain_tip_id_opt.is_none() {
                    log::error!("Longest chain in memory does not match database.");
                    anyhow::bail!("Longest chain in memory does not match database.")
                } else {
                    let db_chain_tip_id_hex = db_chain_tip_id_opt.unwrap();
                    let mem_chain_tip_id_hex =
                        hex::encode(longest_chain.get_tip().unwrap().id().to_vec());
                    if db_chain_tip_id_hex != mem_chain_tip_id_hex {
                        // TODO: Load only new block headers
                        longest_chain = MineLch::get_longest_chain(&pool).await?;
                    }
                }
            }

            let chain_length = longest_chain.headers.len();
            if chain_length != building_block_num {
                building_block_num = chain_length;
                info!("Building block: {}", building_block_num);
            }
        }

        // 2. Any new blocks that need to be voted on? Gather votes for new
        //    blocks, mark votes as valid or invalid. Update longest chain if
        //    votes are valid. Continue main loop if found, because everything
        //    hinges on whether a new block is found to be valid.
        {
            let new_mine_headers = MineHeader::get_voting_headers(&pool).await?;
            debug!("New voting headers: {}", new_mine_headers.len());
            for new_mine_header in &new_mine_headers {
                // if votes are valid, mark as such
                // TODO: Gather and assess votes
                // TODO: This should be a transaction
                MineHeader::update_is_vote_valid(&new_mine_header.id, true, &pool).await?;
                let mine_lch = MineLch::from_mine_header(new_mine_header);
                let res = mine_lch.save(&pool).await;
                if let Err(e) = res {
                    error!("Failed to save new block header: {}", e);
                    anyhow::bail!("Failed to save new block header: {}", e)
                }
                info!("New longest chain tip ID: {}", mine_lch.id);
                continue 'main_loop;
            }
        }

        // 3. Validate new block, broadcast, and continue main loop if found.
        {
            let new_mine_headers = MineHeader::get_validated_headers(&pool).await?;
            debug!("New validated headers: {}", new_mine_headers.len());
            for new_mine_header in &new_mine_headers {
                let header = new_mine_header.to_block_header();
                let _ = header;
                // TODO: Verify block
                MineHeader::update_is_block_valid(&new_mine_header.id, true, &pool).await?;
                info!(
                    "New validated block ID: {}",
                    Buffer::from(header.id().to_vec()).to_hex()
                );
                continue 'main_loop;
            }
        }

        // 4. Check for valid PoW and continue main loop if found.
        {
            let new_mine_headers = MineHeader::get_candidate_headers(&pool).await?;
            // for each header, validate against the longest chain
            // if any header is valid, mark as such and continue main loop
            // if header is invalid, mark as such
            debug!("New candidate headers: {}", new_mine_headers.len());
            for new_mine_header in &new_mine_headers {
                let header = new_mine_header.to_block_header();
                if longest_chain.new_header_is_valid_now(&header) {
                    info!(
                        "New header is valid: {}, {}",
                        header.block_num,
                        Buffer::from(header.id().to_vec()).to_hex()
                    );
                    MineHeader::update_is_header_valid(&new_mine_header.id, true, &pool).await?;
                    continue 'main_loop;
                } else {
                    debug!(
                        "Header is invalid: {}, {}",
                        header.block_num,
                        Buffer::from(header.id().to_vec()).to_hex()
                    );
                    debug!(
                        "Header target: {}",
                        Buffer::from(header.target.to_vec()).to_hex()
                    );
                    MineHeader::update_is_header_valid(&new_mine_header.id, false, &pool).await?;
                }
            }
        }

        // 5. Check for new transactions and validate. Broadcast if found.
        {}

        // 6. Create new candidate block header for mining.
        {
            // produce and insert coinbase transaction
            let coinbase_tx: Tx;
            {
                coinbase_tx = longest_chain
                    .get_next_coinbase_tx(&config.coinbase_pkh, &config.domain.clone());
                let coinbase_tx_id = hex::encode(coinbase_tx.id().to_vec());
                debug!("Coinbase tx ID: {}", coinbase_tx_id);
                let coinbase_mine_tx = MineTxParsed::get(&coinbase_tx_id, &pool).await;
                if let Err(_) = coinbase_mine_tx {
                    info!("Inserting coinbase tx ID: {}", coinbase_tx_id);
                    let ebx_address: Option<String> = None;
                    let res_tx_id = MineTxRaw::parse_and_insert(
                        &coinbase_tx,
                        config.domain.clone(),
                        ebx_address,
                        &pool,
                    )
                    .await;
                    if let Err(e) = res_tx_id {
                        error!("Failed to insert coinbase tx: {}", e);
                        anyhow::bail!("Failed to insert coinbase tx: {}", e)
                    }
                } else {
                    debug!("Coinbase tx already exists: {}", coinbase_tx_id);
                }
            }

            // TODO: Get (synchronize) all unconfirmed transactions (mempool)
            let mempool_txs: Vec<Tx> = vec![];

            // combine coinbase and mempool transactions
            let unconfirmed_txs: Vec<Tx> =
                vec![coinbase_tx].into_iter().chain(mempool_txs).collect();

            // Produce Merkle root and Merkle proofs
            let merkle_txs = MerkleTxs::new(unconfirmed_txs);
            let merkle_root: [u8; 32] = merkle_txs.root;

            // Save all Merkle proofs (upsert)
            {
                for (tx, proof) in merkle_txs.get_iterator() {
                    // TODO: This should be a transaction
                    let mine_merkle_proof =
                        MineMerkleProof::from_merkle_proof(&proof, tx.id().to_vec());
                    let res = mine_merkle_proof.upsert(&pool).await;
                    if let Err(e) = res {
                        error!("Failed to upsert merkle proof: {}", e);
                        anyhow::bail!("Failed to upsert merkle proof: {}", e)
                    }
                }
            }

            // Produce candidate header
            let new_timestamp = Header::get_new_timestamp();
            let header = match longest_chain.get_next_header(merkle_root, new_timestamp) {
                Ok(header) => header,
                Err(e) => {
                    error!("Failed to produce header: {}", e);
                    anyhow::bail!("Failed to produce header: {}", e)
                }
            };
            let block_id = header.id();

            // Save candidate header
            let mine_header = MineHeader::from_header(&header, config.domain.clone());
            let res = MineHeader::get(&mine_header.id, &pool).await;
            if let Ok(_) = res {
                // this can hypothetically happen if timestamp is the same
                debug!(
                    "Candidate header already exists: {}",
                    Buffer::from(block_id.to_vec()).to_hex()
                );
            } else {
                mine_header.save(&pool).await?;
                debug!(
                    "Produced candidate header ID: {}",
                    Buffer::from(block_id.to_vec()).to_hex()
                );
            }
        }

        // 7. Clean up old headers, merkle proofs, coinbase transactions, etc.
        {
            // Delete old unused block headers
            let res = MineHeader::delete_unused_headers(building_block_num as u64, &pool).await;
            if let Err(e) = res {
                error!("Failed to delete unused headers: {}", e);
                anyhow::bail!("Failed to delete unused headers: {}", e)
            } else {
                debug!(
                    "Deleted unused headers before block num: {}",
                    building_block_num
                );
            }
            // TODO: Delete old unused merkle proofs
            // TODO: Delete old unused coinbase transactions
            // TODO: Any other cleanup processes
        }
        interval.tick().await;
    }
}