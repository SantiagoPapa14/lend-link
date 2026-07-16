use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    block::{ClaimBlock, Hash},
    storage::BlockStorage,
};

#[derive(Serialize, Deserialize)]
pub struct ChainLink {
    pub depth: u32,
    pub block: ClaimBlock,
    pub children: Vec<Hash>,
}

#[derive(Serialize, Deserialize)]
pub struct ClaimChain<T: BlockStorage> {
    pub links: HashMap<Hash, ChainLink>,
    pub genesis: Option<Hash>,
    pub orphans_by_parent: HashMap<Hash, Vec<ClaimBlock>>,

    #[serde(skip)]
    storage: T,
}

impl<T: BlockStorage> ClaimChain<T> {
    pub fn new(block_storage: T) -> ClaimChain<T> {
        ClaimChain {
            links: HashMap::new(),
            genesis: None,
            orphans_by_parent: HashMap::new(),
            storage: block_storage,
        }
    }

    pub fn get_tip(&self) -> Result<&ClaimBlock, String> {
        let mut best: Option<(&Hash, &ChainLink)> = None;

        for (hash, link) in &self.links {
            if !link.children.is_empty() {
                continue;
            }
            best = match best {
                None => Some((hash, link)),
                Some((best_hash, best_link)) => {
                    if link.depth > best_link.depth
                        || (link.depth == best_link.depth && hash < best_hash)
                    {
                        Some((hash, link))
                    } else {
                        Some((best_hash, best_link))
                    }
                }
            };
        }
        let (_, link) = best.ok_or("Did not find a suitable tip")?;
        return Ok(&link.block);
    }

    fn reprocess_orphans_by_parent(&mut self, parent: Hash) -> Result<(), String> {
        if let Some(claims) = self.orphans_by_parent.remove(&parent) {
            for claim in claims {
                let _ = self.add_claim(claim)?;
            }
        }
        Ok(())
    }

    pub fn add_claim(&mut self, claim: ClaimBlock) -> Result<(), String> {
        let claim_hash = claim.hash.clone();
        if self.links.len() > 0 {
            if claim.previous_hash.is_none() {
                return Err("Previous hash must be set".to_string());
            }
            if !claim.validate() {
                return Err("Invalid claim".to_string());
            }

            let parent = self
                .links
                .get_mut(claim.previous_hash.as_ref().expect("No previous hash"));

            if parent.is_none() {
                let orphans = self
                    .orphans_by_parent
                    .get_mut(&claim.previous_hash.clone().unwrap());

                let mut to_append = vec![claim.clone()];
                if orphans.is_some() {
                    let orphans = orphans.unwrap();
                    orphans.append(&mut to_append);
                } else {
                    self.orphans_by_parent
                        .insert(claim.previous_hash.clone().unwrap(), to_append);
                }

                self.storage.save(&claim, "orphan".to_string())?;
                return Ok(());
            }

            let parent = parent.unwrap();

            parent.children.push(claim_hash.clone());
            let link = ChainLink {
                depth: parent.depth.clone() + 1,
                block: claim.clone(),
                children: Vec::new(),
            };
            self.links.insert(claim_hash.clone(), link);
            self.storage.save(&claim, self.genesis.clone().unwrap())?;
            self.reprocess_orphans_by_parent(claim_hash)?;
            Ok(())
        } else {
            if !claim.validate() {
                return Err("Invalid claim".to_string());
            }
            if claim.previous_hash.is_some() {
                return Err("Previous hash must be None for genesis".to_string());
            }

            self.genesis = Some(claim.hash.clone());
            let link = ChainLink {
                depth: 0,
                block: claim.clone(),
                children: Vec::new(),
            };
            self.links.insert(claim_hash.clone(), link);
            self.storage.save(&claim, self.genesis.clone().unwrap())?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
    };

    use crate::{
        block::{Address, ClaimBlock, Hash, Issuer},
        crypto,
        storage::{BlockStorage, MemoryStorage},
    };
    use ed25519_dalek::SigningKey;

    use super::ClaimChain;

    struct RecordingStorage {
        chain_ids: RefCell<HashMap<Hash, Hash>>,
    }

    impl BlockStorage for RecordingStorage {
        fn init() -> Result<Self, String> {
            Ok(Self {
                chain_ids: RefCell::new(HashMap::new()),
            })
        }

        fn save(&self, claim: &ClaimBlock, chain_id: Hash) -> Result<(), String> {
            self.chain_ids
                .borrow_mut()
                .insert(claim.hash.clone(), chain_id);
            Ok(())
        }
    }

    struct FailOnSaveStorage {
        save_count: Cell<usize>,
        fail_on: usize,
    }

    impl BlockStorage for FailOnSaveStorage {
        fn init() -> Result<Self, String> {
            Ok(Self {
                save_count: Cell::new(0),
                fail_on: usize::MAX,
            })
        }

        fn save(&self, _claim: &ClaimBlock, _chain_id: Hash) -> Result<(), String> {
            let next = self.save_count.get() + 1;
            self.save_count.set(next);

            if next == self.fail_on {
                Err("injected storage failure".to_string())
            } else {
                Ok(())
            }
        }
    }

    fn new_chain() -> ClaimChain<MemoryStorage> {
        let storage = MemoryStorage::init().expect("in-memory storage should initialize");
        ClaimChain::new(storage)
    }

    fn signed_claim(
        previous_hash: Option<String>,
        issuer: Issuer,
        amount: i32,
        lender_private: &SigningKey,
        lender_public: Address,
        borrower_private: &SigningKey,
        borrower_public: Address,
    ) -> ClaimBlock {
        let claim = ClaimBlock {
            previous_hash,
            hash: String::new(),
            issuer,
            lender: lender_public,
            borrower: borrower_public,
            amount,
            issued_at: "2026-07-10T00:00:00Z".to_string(),
            issuer_signature: None,
        };

        let hash = claim.calculate_hash();
        let signing_key = match claim.issuer {
            Issuer::Lender => lender_private,
            Issuer::Borrower => borrower_private,
            Issuer::Genesis => borrower_private,
        };
        let signature = crypto::sign_message(signing_key, hash.as_bytes());

        ClaimBlock {
            hash,
            issuer_signature: Some(signature),
            ..claim
        }
    }

    #[test]
    fn empty_chain_has_no_tip() {
        let chain = new_chain();

        assert!(chain.get_tip().is_err());
    }

    #[test]
    fn first_valid_claim_is_accepted() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();
        let claim = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );

        assert!(chain.add_claim(claim).is_ok());
        assert_eq!(chain.links.len(), 1);
        assert_eq!(chain.genesis.as_ref(), chain.links.keys().next());
    }

    #[test]
    fn orphan_claim_is_indexed_by_its_missing_parent() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let first = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        chain.add_claim(first).unwrap();

        let orphan_parent = "missing-parent".to_string();
        let orphan = signed_claim(
            Some(orphan_parent.clone()),
            Issuer::Lender,
            15,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );

        assert!(chain.add_claim(orphan.clone()).is_ok());
        assert_eq!(chain.links.len(), 1);
        assert_eq!(chain.orphans_by_parent.len(), 1);
        assert!(chain.orphans_by_parent.contains_key(&orphan_parent));
        assert_eq!(
            chain.orphans_by_parent.get(&orphan_parent).unwrap().len(),
            1
        );
        assert_eq!(
            chain.orphans_by_parent.get(&orphan_parent).unwrap()[0].hash,
            orphan.hash
        );
    }

    #[test]
    fn second_valid_claim_is_accepted() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let first = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let first_hash = first.hash.clone();
        chain.add_claim(first).unwrap();

        let second = signed_claim(
            Some(first_hash),
            Issuer::Lender,
            15,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );

        assert!(chain.add_claim(second).is_ok());
        assert_eq!(chain.links.len(), 2);
    }

    #[test]
    fn non_genesis_claim_without_previous_hash_is_rejected_without_panicking() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let genesis = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        chain.add_claim(genesis).unwrap();

        let invalid = signed_claim(
            None,
            Issuer::Lender,
            15,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );

        assert!(chain.add_claim(invalid).is_err());
        assert_eq!(chain.links.len(), 1);
    }

    #[test]
    fn tip_of_single_chain_is_the_only_leaf() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let genesis = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let genesis_hash = genesis.hash.clone();
        chain.add_claim(genesis).unwrap();

        let child = signed_claim(
            Some(genesis_hash),
            Issuer::Lender,
            15,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );
        let child_hash = child.hash.clone();
        chain.add_claim(child).unwrap();

        let tip = chain.get_tip().unwrap();
        assert_eq!(tip.hash, child_hash);
    }

    #[test]
    fn single_genesis_chain_tip_is_genesis() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let genesis = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );
        let genesis_hash = genesis.hash.clone();
        chain.add_claim(genesis).unwrap();

        let tip = chain.get_tip().unwrap();
        assert_eq!(tip.hash, genesis_hash);
    }

    #[test]
    fn tip_chooses_deeper_leaf_over_shallower_leaf() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let genesis = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let genesis_hash = genesis.hash.clone();
        chain.add_claim(genesis).unwrap();

        let shallow = signed_claim(
            Some(genesis_hash.clone()),
            Issuer::Lender,
            15,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let shallow_hash = shallow.hash.clone();
        chain.add_claim(shallow).unwrap();

        let deeper = signed_claim(
            Some(shallow_hash),
            Issuer::Borrower,
            20,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );
        let deeper_hash = deeper.hash.clone();
        chain.add_claim(deeper).unwrap();

        let tip = chain.get_tip().unwrap();
        assert_eq!(tip.hash, deeper_hash);
    }

    #[test]
    fn tip_breaks_ties_by_smallest_hash() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let genesis = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let genesis_hash = genesis.hash.clone();
        chain.add_claim(genesis).unwrap();

        let left = signed_claim(
            Some(genesis_hash.clone()),
            Issuer::Lender,
            15,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let right = signed_claim(
            Some(genesis_hash),
            Issuer::Lender,
            25,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );

        let expected = if left.hash < right.hash {
            left.hash.clone()
        } else {
            right.hash.clone()
        };

        chain.add_claim(left).unwrap();
        chain.add_claim(right).unwrap();

        let tip = chain.get_tip().unwrap();
        assert_eq!(tip.hash, expected);
    }

    #[test]
    fn forks_preserve_both_children_of_the_same_parent() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let genesis = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let genesis_hash = genesis.hash.clone();
        chain.add_claim(genesis).unwrap();

        let left = signed_claim(
            Some(genesis_hash.clone()),
            Issuer::Lender,
            15,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let right = signed_claim(
            Some(genesis_hash),
            Issuer::Borrower,
            25,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );

        let left_hash = left.hash.clone();
        let right_hash = right.hash.clone();

        chain.add_claim(left).unwrap();
        chain.add_claim(right).unwrap();

        let genesis_link = chain
            .links
            .values()
            .find(|link| link.depth == 0)
            .expect("genesis link not found");

        assert_eq!(chain.links.len(), 3);
        assert!(genesis_link.children.contains(&left_hash));
        assert!(genesis_link.children.contains(&right_hash));
        assert_eq!(genesis_link.children.len(), 2);
    }

    #[test]
    fn orphan_is_attached_after_its_parent_arrives() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let genesis = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let genesis_hash = genesis.hash.clone();
        chain.add_claim(genesis).unwrap();

        let parent = signed_claim(
            Some(genesis_hash.clone()),
            Issuer::Lender,
            15,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let parent_hash = parent.hash.clone();

        let orphan = signed_claim(
            Some(parent_hash.clone()),
            Issuer::Borrower,
            20,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );
        let orphan_hash = orphan.hash.clone();

        chain
            .orphans_by_parent
            .entry(parent_hash.clone())
            .or_default()
            .push(orphan.clone());

        chain.add_claim(parent).unwrap();

        assert!(
            !chain.orphans_by_parent.contains_key(&parent_hash),
            "orphan should have been resolved once the parent arrived"
        );
        assert!(
            chain.links.contains_key(&orphan_hash),
            "orphan should be attached to the main chain once the parent arrives"
        );
    }

    #[test]
    fn orphan_chain_resolves_in_sequence_when_parent_unlocks_child() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let genesis = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let genesis_hash = genesis.hash.clone();
        chain.add_claim(genesis).unwrap();

        let child = signed_claim(
            Some(genesis_hash.clone()),
            Issuer::Lender,
            15,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let child_hash = child.hash.clone();

        let grandchild = signed_claim(
            Some(child_hash.clone()),
            Issuer::Borrower,
            20,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );
        let grandchild_hash = grandchild.hash.clone();

        chain
            .orphans_by_parent
            .entry(genesis_hash.clone())
            .or_default()
            .push(child.clone());
        chain
            .orphans_by_parent
            .entry(child_hash.clone())
            .or_default()
            .push(grandchild.clone());

        let _ = chain.reprocess_orphans_by_parent(genesis_hash.clone());

        assert!(chain.links.contains_key(&child_hash));
        assert!(
            chain.links.contains_key(&grandchild_hash),
            "grandchild should be attached once the child is resolved"
        );
        assert!(!chain.orphans_by_parent.contains_key(&genesis_hash));
        assert!(!chain.orphans_by_parent.contains_key(&child_hash));
    }

    #[test]
    fn forked_orphans_with_same_parent_are_all_resolved_when_parent_arrives() {
        let mut chain = new_chain();
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let genesis = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let genesis_hash = genesis.hash.clone();
        chain.add_claim(genesis).unwrap();

        let parent = signed_claim(
            Some(genesis_hash.clone()),
            Issuer::Lender,
            15,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let parent_hash = parent.hash.clone();

        let left_orphan = signed_claim(
            Some(parent_hash.clone()),
            Issuer::Borrower,
            20,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let left_hash = left_orphan.hash.clone();

        let right_orphan = signed_claim(
            Some(parent_hash.clone()),
            Issuer::Lender,
            25,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );
        let right_hash = right_orphan.hash.clone();

        chain
            .orphans_by_parent
            .entry(parent_hash.clone())
            .or_default()
            .push(left_orphan);
        chain
            .orphans_by_parent
            .entry(parent_hash.clone())
            .or_default()
            .push(right_orphan);

        chain.add_claim(parent).unwrap();

        assert!(chain.links.contains_key(&parent_hash));
        assert!(chain.links.contains_key(&left_hash));
        assert!(chain.links.contains_key(&right_hash));
        assert!(!chain.orphans_by_parent.contains_key(&parent_hash));

        let parent_link = chain.links.get(&genesis_hash).expect("genesis not found");
        assert_eq!(parent_link.children.len(), 1);
        assert!(parent_link.children.contains(&parent_hash));
    }

    #[test]
    fn resolving_an_orphan_updates_its_persisted_chain_id() {
        let storage = RecordingStorage::init().unwrap();
        let mut chain = ClaimChain::new(storage);
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let genesis = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let genesis_hash = genesis.hash.clone();
        chain.add_claim(genesis).unwrap();

        let parent = signed_claim(
            Some(genesis_hash.clone()),
            Issuer::Lender,
            15,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let parent_hash = parent.hash.clone();
        let orphan = signed_claim(
            Some(parent_hash),
            Issuer::Borrower,
            20,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );
        let orphan_hash = orphan.hash.clone();

        chain.add_claim(orphan).unwrap();
        assert_eq!(
            chain.storage.chain_ids.borrow().get(&orphan_hash),
            Some(&"orphan".to_string())
        );

        chain.add_claim(parent).unwrap();

        assert!(chain.links.contains_key(&orphan_hash));
        assert_eq!(
            chain.storage.chain_ids.borrow().get(&orphan_hash),
            Some(&genesis_hash)
        );
    }

    #[test]
    fn failed_orphan_reprocessing_keeps_unprocessed_siblings_queued() {
        let storage = FailOnSaveStorage {
            save_count: Cell::new(0),
            fail_on: 5,
        };
        let mut chain = ClaimChain::new(storage);
        let (lender_private, lender_public) = crypto::generate_keys();
        let (borrower_private, borrower_public) = crypto::generate_keys();

        let genesis = signed_claim(
            None,
            Issuer::Borrower,
            10,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let genesis_hash = genesis.hash.clone();
        chain.add_claim(genesis).unwrap();

        let parent = signed_claim(
            Some(genesis_hash),
            Issuer::Lender,
            15,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let parent_hash = parent.hash.clone();

        let first_orphan = signed_claim(
            Some(parent_hash.clone()),
            Issuer::Borrower,
            20,
            &lender_private,
            lender_public.clone(),
            &borrower_private,
            borrower_public.clone(),
        );
        let second_orphan = signed_claim(
            Some(parent_hash.clone()),
            Issuer::Lender,
            25,
            &lender_private,
            lender_public,
            &borrower_private,
            borrower_public,
        );
        let second_orphan_hash = second_orphan.hash.clone();

        chain.add_claim(first_orphan).unwrap();
        chain.add_claim(second_orphan).unwrap();

        assert!(chain.add_claim(parent).is_err());
        assert!(
            chain
                .orphans_by_parent
                .get(&parent_hash)
                .is_some_and(|claims| claims.iter().any(|claim| claim.hash == second_orphan_hash)),
            "an orphan not yet processed should remain queued after its sibling fails"
        );
    }
}
